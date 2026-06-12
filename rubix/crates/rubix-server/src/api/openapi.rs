//! OpenAPI document assembly; served at `/api-docs/openapi.json`.

use axum::Json;
use utoipa::OpenApi;

use crate::error::ErrorBody;

use super::{command, equips, health, his, points, sites, sparks};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rubix BMS API",
        description = "Supervisory backend: sites, equips, points (priority-array command), \
                       history, spark findings."
    ),
    paths(
        health::healthz,
        sites::create::create_site,
        sites::list::list_sites,
        sites::get::get_site,
        sites::delete::delete_site,
        equips::create::create_equip,
        equips::list::list_equips,
        equips::get::get_equip,
        equips::delete::delete_equip,
        points::create::create_point,
        points::list::list_points,
        points::get::get_point,
        points::delete::delete_point,
        command::write::write_point,
        command::relinquish::relinquish_point,
        command::cur::ingest_cur,
        his::query::query_his,
        his::insert::insert_his,
        sparks::create::create_spark,
        sparks::list::list_sparks,
        sparks::ack::ack_spark,
    ),
    components(schemas(ErrorBody))
)]
pub struct ApiDoc;

pub(super) async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
