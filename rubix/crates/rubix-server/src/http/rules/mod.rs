//! Rule resource routes — author, list, debug, and compose automation rules.
//!
//! A rule is the deterministic decision unit (`rubix/docs/SCOPE.md`, "Rhai —
//! rules and insights"): a Rhai script over time-window bindings that emits an
//! insight. Rules persist as `kind:"rule"` records, so mutations cross the WS-05
//! gate (on the [`RuleDefine`](rubix_gate::Capability::RuleDefine) grant) and reads
//! run on the WS-03 scoped session, exactly like any record — no new table. On top
//! of CRUD this module adds the two surfaces the rules studio needs: a
//! side-effect-free **dry-run** (run a draft against real history without firing)
//! and a **referencing** read (which rules compose this one — the blast radius
//! before an edit/delete). One file per route; this barrel only merges them.

mod capability;
mod create;
mod delete;
mod dryrun;
mod get;
mod list;
mod referencing;
mod shared;
mod update;
mod validate;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

use create::create_rule_route;
use delete::delete_rule_route;
use dryrun::dryrun_rule_route;
use get::get_rule_route;
use list::list_rules_route;
use referencing::referencing_rules_route;
use update::update_rule_route;

/// The rule routes mounted under `/rules`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/rules", post(create_rule_route).get(list_rules_route))
        .route(
            "/rules/:name",
            get(get_rule_route)
                .patch(update_rule_route)
                .delete(delete_rule_route),
        )
        .route("/rules/:name/dryrun", post(dryrun_rule_route))
        .route("/rules/:name/referencing", get(referencing_rules_route))
}
