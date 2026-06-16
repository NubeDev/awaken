//! `GET /extensions/<id>/events` — the supervisor's bounded event ring.
//!
//! Paginated from a `?after=<seq>` cursor: returns the retained ring events whose
//! sequence is strictly greater than `after`, plus the `next_seq` cursor to
//! resume from. This is the resumable foundation an SSE live-tail upgrades on top
//! of (the `seq` cursor is monotone across reconnects and survives ring
//! eviction); the live-tail `Accept: text/event-stream` upgrade is a follow-on
//! increment over this same cursor and is intentionally not wired yet.
//!
//! An extension with a live supervisor returns its ring; one with only a control
//! record (stopped, never started) returns an empty ring at cursor 0; an unknown
//! id is a `404`.

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};

use rubix_ext::supervisor::Event;

use crate::auth::Authenticated;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::shared::{ext_id, find_control_record};

/// The `?after=<seq>` pagination cursor.
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    /// Return only events whose `seq` is strictly greater than this. Absent means
    /// from the start of the retained ring.
    #[serde(default)]
    after: Option<u64>,
}

/// The `GET /extensions/<id>/events` body.
#[derive(Debug, Serialize)]
pub struct EventsPage {
    /// The retained ring events after the cursor, oldest first.
    pub events: Vec<Event>,
    /// The cursor the next request should pass as `after` to resume.
    pub next_seq: u64,
}

/// Read a page of an extension's supervisor event ring.
pub async fn events_extension_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(subject): Path<String>,
    Query(query): Query<EventsQuery>,
) -> ApiResult<Json<EventsPage>> {
    // The control record must exist for any answer; an unknown id is a 404.
    find_control_record(&auth.session, &subject)
        .await?
        .ok_or(ApiError::NotFound)?;
    let id = ext_id(&auth, &subject);

    let page = match state.extensions.supervisors.get(&id) {
        Some(handle) => {
            let events = match query.after {
                Some(after) => handle.events_since(after),
                None => handle.events(),
            };
            EventsPage {
                events,
                next_seq: handle.events_next_seq(),
            }
        }
        None => EventsPage {
            events: Vec::new(),
            next_seq: 0,
        },
    };
    Ok(Json(page))
}
