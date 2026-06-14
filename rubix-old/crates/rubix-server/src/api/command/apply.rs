//! Shared command path: apply a priority-array change, then publish the new
//! `cur` value on the bus. Used by `write` and `relinquish`.

use chrono::Utc;
use rubix_core::PointValue;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::api::points::response::PointResponse;
use crate::error::ApiError;
use crate::AppState;

/// Set (`Some`) or relinquish (`None`) a priority slot, return the point with
/// its keyexpr, and publish the resulting effective value on `{keyexpr}/cur`.
pub(super) async fn command_and_publish(
    state: &AppState,
    id: Uuid,
    priority: u8,
    value: Option<PointValue>,
) -> Result<PointResponse, ApiError> {
    let store = state.store.clone();
    let response = blocking(move || {
        let point = store.command_point(id, priority, value, Utc::now())?;
        let keyexpr = store.point_keyexpr(id)?;
        Ok(PointResponse { keyexpr, point })
    })
    .await?;
    if let Some(bus) = &state.bus {
        bus.publish_cur(&response.keyexpr, response.point.cur_value.as_ref())
            .await;
    }
    Ok(response)
}
