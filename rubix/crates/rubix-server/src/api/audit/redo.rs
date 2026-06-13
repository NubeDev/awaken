//! POST /api/v1/redo — re-apply the authenticated principal's most-recently-undone
//! change group (docs/design/audit-and-undo.md "Undo/Redo"). Pops the top of the
//! actor's redo stack, replays its rows forward through the reverser registry, and
//! CAS-removes the group from the stack. Per-actor and CAS-guarded, mirroring undo.

use axum::extract::State;
use axum::Json;

use super::dispatch::{cursor_subject, touched_ids};
use super::undo::{UndoRequest, UndoResult};
use crate::api::audit::record::actor_of;
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::store::{apply_group_forward, ReverserRegistry};
use crate::AppState;

#[utoipa::path(post, path = "/api/v1/redo", request_body = UndoRequest, tag = "audit",
    security(("bearer" = [])),
    responses((status = 200, body = UndoResult), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody)))]
pub(crate) async fn redo(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<UndoRequest>,
) -> Result<Json<UndoResult>, ApiError> {
    let actor = actor_of(&principal);
    let subject = cursor_subject(&actor);
    let org = req.org;
    let store = state.store.clone();

    let result = blocking(move || {
        let registry = ReverserRegistry::new();
        let cursor = store.undo_cursor(&org, &subject)?;
        let mut stack = cursor.redo_stack.clone();
        let Some(group) = stack.pop() else {
            return Ok(UndoResult { group: None, touched: Vec::new() });
        };
        let rows = store.changes_in_group(group)?;
        apply_group_forward(&store, &registry, &rows)?;

        if !store.cas_undo_cursor(&org, &subject, cursor.epoch, &stack)? {
            return Err(ApiError::Conflict(
                "a concurrent redo advanced the cursor; retry".into(),
            ));
        }
        Ok(UndoResult { group: Some(group), touched: touched_ids(&rows) })
    })
    .await?;
    Ok(Json(result))
}
