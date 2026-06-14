//! POST /api/v1/undo — undo the authenticated principal's most-recent change group
//! (docs/design/audit-and-undo.md "Undo/Redo"). Per-actor and CAS-guarded: it pops
//! the actor's newest not-yet-undone group, replays its rows in reverse through the
//! reverser registry, and pushes the group onto the actor's redo stack. Undo targets
//! *your* changes only — cross-actor global undo is an explicit non-goal. Returns the
//! affected `group_id` and the touched resource ids so the UI invalidates exactly
//! those queries.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::dispatch::{cursor_subject, touched_ids};
use crate::api::audit::record::actor_of;
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::store::{apply_group_inverse, ReverserRegistry};
use crate::AppState;

/// The tenant whose log to undo within. The cursor is per-actor and per-org, so the
/// request names the org the change was made in.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UndoRequest {
    pub org: String,
}

/// The result of an undo/redo: the group that moved and the resource ids it touched
/// (so the UI invalidates exactly the matching query keys). `group` is `None` when
/// there was nothing to undo/redo.
#[derive(Debug, Serialize, ToSchema)]
pub struct UndoResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<Uuid>,
    pub touched: Vec<Uuid>,
}

#[utoipa::path(post, path = "/api/v1/undo", request_body = UndoRequest, tag = "audit",
    security(("bearer" = [])),
    responses((status = 200, body = UndoResult), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody)))]
pub(crate) async fn undo(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<UndoRequest>,
) -> Result<Json<UndoResult>, ApiError> {
    // Undo acts on the caller's own changes; require an authenticated caller and
    // attribute to their cursor key.
    let actor = actor_of(&principal);
    let subject = cursor_subject(&actor);
    let org = req.org;
    let store = state.store.clone();

    let result = blocking(move || {
        let registry = ReverserRegistry::new();
        let Some(group) = store.newest_undoable_group(&org, &subject)? else {
            return Ok(UndoResult { group: None, touched: Vec::new() });
        };
        let rows = store.changes_in_group(group)?;
        apply_group_inverse(&store, &registry, &rows)?;

        // CAS the group onto the actor's redo stack; a racing undo that advanced the
        // epoch makes this a no-op and we report the conflict (the double-pop guard).
        let cursor = store.undo_cursor(&org, &subject)?;
        let mut stack = cursor.redo_stack.clone();
        stack.push(group);
        if !store.cas_undo_cursor(&org, &subject, cursor.epoch, &stack)? {
            return Err(ApiError::Conflict(
                "a concurrent undo advanced the cursor; retry".into(),
            ));
        }
        Ok(UndoResult { group: Some(group), touched: touched_ids(&rows) })
    })
    .await?;
    Ok(Json(result))
}
