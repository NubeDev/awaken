//! Response type shared by point and command verbs.

use rubix_core::Point;
use serde::Serialize;
use utoipa::ToSchema;

/// A point plus its zenoh keyexpr identity (`{org}/{site}/{equip-path}/{point}`).
#[derive(Debug, Serialize, ToSchema)]
pub struct PointResponse {
    pub keyexpr: String,
    pub point: Point,
}
