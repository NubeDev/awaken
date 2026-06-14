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
use std::time::{Duration, Instant};

use reflow_actor::message::Message;
use reflow_actor::ActorContext;

mod trigger_state;

use crate::node::actor_base::{boxed, error_out, ActorBase};
use crate::port::PointAccess;
use crate::rubix_node;

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
            // The trigger's clock state lives in `trigger_state`, not the seam,
            // so the body does no async I/O — wrap the sync `fire` in a ready
            // future to satisfy the async `NodeBody`.
            body: Arc::new(|_access, context| boxed(async move { fire(context, Instant::now()) })),
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

/// Evaluate the trigger for one invocation against clock reading `now`.
fn fire(context: &ActorContext, now: Instant) -> HashMap<String, Message> {
    let period = match period(context) {
        Ok(p) => p,
        Err(e) => return error_out(e),
    };
    let node_id = context.get_config().get_node_id();
    match trigger_state::advance(node_id, period, now) {
        None => HashMap::new(), // not due yet — stay quiet
        Some(f) => HashMap::from([
            ("boot".to_string(), Message::Boolean(f.boot)),
            ("count".to_string(), Message::Integer(f.count)),
            ("output".to_string(), Message::Boolean(f.level)),
        ]),
    }
}

rubix_node!(TriggerActor);
