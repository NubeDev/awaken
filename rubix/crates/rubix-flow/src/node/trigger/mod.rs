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

/// Actor-state key under which the trigger's retained slot is stored.
const SLOT_KEY: &str = "trigger.slot";

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
            // The trigger's clock state lives in the actor's own state (which
            // survives across scans on the persistent engine), not the seam, so
            // the body does no async I/O — wrap the sync `fire` in a ready future
            // to satisfy the async `NodeBody`.
            body: Arc::new(|_access, context| boxed(async move { fire(context, now_ms()) })),
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

/// Evaluate the trigger for one invocation against wall-clock `now_ms`. Reads and
/// writes the retained slot from the actor's own state, which persists across
/// scans on the persistent engine.
fn fire(context: &ActorContext, now_ms: i64) -> HashMap<String, Message> {
    let period = match period(context) {
        Ok(p) => p,
        Err(e) => return error_out(e),
    };
    let period_ms = period.as_millis() as i64;
    let mut slot = load_slot(context);
    let fired = trigger_state::advance(&mut slot, period_ms, now_ms);
    store_slot(context, slot);
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

/// Load the retained slot from the actor's state, defaulting to a fresh slot.
fn load_slot(context: &ActorContext) -> TriggerSlot {
    let state = context.get_state();
    let guard = state.lock();
    guard
        .as_any()
        .downcast_ref::<MemoryState>()
        .and_then(|mem| mem.get(SLOT_KEY))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// Persist the slot back to the actor's state for the next scan.
fn store_slot(context: &ActorContext, slot: TriggerSlot) {
    let state = context.get_state();
    let mut guard = state.lock();
    if let Some(mem) = guard.as_mut_any().downcast_mut::<MemoryState>() {
        if let Ok(value) = serde_json::to_value(slot) {
            mem.insert(SLOT_KEY, value);
        }
    }
}

rubix_node!(TriggerActor);
