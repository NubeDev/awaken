//! Dev-seed one stored board so the Flows surface renders a real graph from
//! `/api/v1/boards` instead of a static fixture. A `Manual` board (the
//! scheduler never fires it) wired as source -> agent -> sink over real seeded
//! ahu-3 points, so `POST /boards/{slug}/run` exercises the real engine.
//!
//! Idempotent by slug: a board whose slug already exists is left untouched, so
//! re-seeding neither duplicates nor errors (matches the rest of the dev seed).

use std::collections::HashMap;

use chrono::Utc;
use rubix_flow::{BoardConnection, BoardGraph, BoardNode};
use serde_json::json;
use uuid::Uuid;

use super::portfolio::ORG;
use super::SeedError;
use crate::scheduler::{BoardRecord, Trigger};
use crate::store::Store;

/// Slug of the seeded demo board. Stable so the seed stays idempotent.
const BOARD_SLUG: &str = "ahu-3-discharge-reset";

/// Site the seeded board points at. The first seeded site; its ahu-3 equip
/// carries the occupancy/cooling-valve points the board reads and commands.
const SITE: &str = "hq-tower";

/// Ensure the demo board exists. Returns 1 if it was newly created, 0 if it
/// already existed (idempotent dev seed).
pub fn seed_board(store: &Store) -> Result<usize, SeedError> {
    if store
        .latest_boards()?
        .iter()
        .any(|b| b.slug == BOARD_SLUG)
    {
        return Ok(0);
    }
    let version = store.next_board_version(BOARD_SLUG)?;
    let board = BoardRecord {
        id: Uuid::new_v4(),
        slug: BOARD_SLUG.to_string(),
        version,
        display_name: "AHU-3 · Discharge Reset".to_string(),
        enabled: true,
        trigger: Trigger::Manual,
        graph: demo_graph(),
        created_at: Utc::now(),
    };
    store.create_board(&board)?;
    Ok(1)
}

/// The wiresheet: read zone occupancy, hand it to the guard agent, command the
/// cooling valve with the agent's verdict. All three components are registered
/// rubix actors (`rubix-flow/src/board/registry.rs`) and the keyexprs resolve
/// to real seeded ahu-3 points.
fn demo_graph() -> BoardGraph {
    let read = BoardNode {
        id: "read-occupancy".to_string(),
        component: "read_point".to_string(),
        config: config([("point", json!(point("occupancy")))]),
    };
    let guard = BoardNode {
        id: "agent-guard".to_string(),
        component: "agent_call".to_string(),
        config: config([(
            "prompt",
            json!("Given zone occupancy, recommend a cooling-valve command for AHU-3."),
        )]),
    };
    let write = BoardNode {
        id: "write-cooling-valve".to_string(),
        component: "write_point".to_string(),
        config: config([("point", json!(point("cooling-valve"))), ("priority", json!(13))]),
    };
    BoardGraph {
        nodes: vec![read, guard, write],
        connections: vec![
            BoardConnection {
                from_node: "read-occupancy".to_string(),
                from_port: "output".to_string(),
                to_node: "agent-guard".to_string(),
                to_port: "value".to_string(),
            },
            BoardConnection {
                from_node: "agent-guard".to_string(),
                from_port: "output".to_string(),
                to_node: "write-cooling-valve".to_string(),
                to_port: "value".to_string(),
            },
        ],
    }
}

/// `{org}/{site}/{equip}/{point}` keyexpr for an ahu-3 point.
fn point(slug: &str) -> String {
    format!("{ORG}/{SITE}/ahu-3/{slug}")
}

fn config<const N: usize>(entries: [(&str, serde_json::Value); N]) -> HashMap<String, serde_json::Value> {
    entries
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}
