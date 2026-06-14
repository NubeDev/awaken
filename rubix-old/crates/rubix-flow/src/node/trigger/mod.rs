//! `trigger` node: a self-paced timing source. The scheduler ticks it (it is a
//! source node — its only inport has no inbound connection), but the node owns
//! its own cadence: it fires only when its configured period (`every` × `unit`)
//! has elapsed since its last fire, independent of how fast the scheduler ticks.
//!
//! On a fire it emits:
//! - `boot`   — `true` only on the first fire after server start, else `false`.
//! - `count`  — total fires since server start (the toggle count).
//! - `output` — a level that toggles on every fire (`true` on the first fire).
//!
//! Between fires it emits nothing, so the board settles with no packets from
//! this node. Period state lives in [`super::trigger_state`], keyed by node id,
//! because each scheduler tick rebuilds the actor fresh.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reflow_actor::message::Message;
use reflow_actor::{ActorContext, MemoryState};

mod trigger_state;

use self::trigger_state::TriggerSlot;
use crate::node::actor_base::{boxed, error_out, ActorBase};
use crate::port::PointAccess;
use crate::rubix_node;
use crate::state::StatePolicy;

/// Sub-key under which the trigger retains its clock slot.
const CLOCK_KEY: &str = "clock";

/// The trigger's clock is `Session` state: it survives a board republish (so
/// saving a board does not restart its timers / re-fire its boot fire), but
/// resets on a server restart (boot fires again after start). A node author
/// chooses this policy explicitly — a different cadence node could pick
/// `Ephemeral` (reset on save) or `Durable` (survive restart) instead.
const TRIGGER_POLICY: StatePolicy = StatePolicy::Session;

#[derive(Clone)]
pub struct TriggerActor {
    pub base: ActorBase,
    pub access: Arc<dyn PointAccess>,
    pub body: crate::node::actor_base::NodeBody,
}

impl TriggerActor {
    pub fn new(access: Arc<dyn PointAccess>) -> Self {
        Self {
            base: ActorBase::new(&["trigger"], &["boot", "count", "output", "error"]),
            access,
            body: Arc::new(|access, context| boxed(fire(access, context))),
        }
    }
}

/// Resolve the configured period from `every` (a positive number, default 1)
/// and `unit` (`sec`|`min`|`hours`, default `sec`).
fn period(context: &ActorContext) -> Result<Duration, String> {
    let every = context.get_config().get_number("every").unwrap_or(1.0);
    if !every.is_finite() || every <= 0.0 {
        return Err(format!("trigger: `every` must be a positive number, got {every}"));
    }
    let unit = context
        .get_config()
        .get_string("unit")
        .unwrap_or_else(|| "sec".to_string());
    let unit_secs = match unit.as_str() {
        "sec" | "secs" | "second" | "seconds" => 1.0,
        "min" | "mins" | "minute" | "minutes" => 60.0,
        "hour" | "hours" | "hr" | "hrs" => 3600.0,
        other => return Err(format!("trigger: unknown `unit` `{other}` (sec|min|hours)")),
    };
    Ok(Duration::from_secs_f64(every * unit_secs))
}

/// Evaluate the trigger for one invocation. Loads and saves its clock slot
/// through the node-state contract under [`TRIGGER_POLICY`], so the clock
/// survives a republish; when the access wires no state store (a test fake, a
/// one-shot Test Run), it falls back to ephemeral actor memory, which resets per
/// run — the correct behaviour there.
async fn fire(access: &Arc<dyn PointAccess>, context: &ActorContext) -> HashMap<String, Message> {
    let period = match period(context) {
        Ok(p) => p,
        Err(e) => return error_out(e),
    };
    let period_ms = period.as_millis() as i64;
    let node = context.get_config().get_node_id().to_string();

    let store = access.node_state();
    let mut slot = load_slot(store.as_deref(), context, &node).await;
    let fired = trigger_state::advance(&mut slot, period_ms, now_ms());
    save_slot(store.as_deref(), context, &node, slot).await;

    match fired {
        None => HashMap::new(), // not due yet — stay quiet
        Some(f) => HashMap::from([
            ("boot".to_string(), Message::Boolean(f.boot)),
            ("count".to_string(), Message::Integer(f.count)),
            ("output".to_string(), Message::Boolean(f.level)),
        ]),
    }
}

/// Current wall-clock time in millis since the Unix epoch (0 if the clock is
/// before the epoch, which never happens in practice).
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Load the clock slot: from the node-state store when present, else from
/// ephemeral actor memory. Defaults to a fresh slot.
async fn load_slot(
    store: Option<&dyn crate::state::NodeState>,
    context: &ActorContext,
    node: &str,
) -> TriggerSlot {
    if let Some(store) = store {
        return store
            .load(node, CLOCK_KEY, TRIGGER_POLICY)
            .await
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
    }
    let state = context.get_state();
    let guard = state.lock();
    guard
        .as_any()
        .downcast_ref::<MemoryState>()
        .and_then(|mem| mem.get(CLOCK_KEY))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// Save the clock slot back to the node-state store, else to ephemeral actor
/// memory.
async fn save_slot(
    store: Option<&dyn crate::state::NodeState>,
    context: &ActorContext,
    node: &str,
    slot: TriggerSlot,
) {
    let value = serde_json::to_value(slot).unwrap_or_default();
    if let Some(store) = store {
        let _ = store.save(node, CLOCK_KEY, TRIGGER_POLICY, value).await;
        return;
    }
    let state = context.get_state();
    let mut guard = state.lock();
    if let Some(mem) = guard.as_mut_any().downcast_mut::<MemoryState>() {
        mem.insert(CLOCK_KEY, value);
    }
}

rubix_node!(TriggerActor);
