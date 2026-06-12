//! OpenAPI document assembly; served at `/api-docs/openapi.json`.

use axum::Json;
use utoipa::OpenApi;

use crate::agent::{PendingWrite, RunOrigin, RunRecord, RunStatus};
use crate::error::ErrorBody;

use super::{
    agent, boards, command, equips, health, his, points, query, runs, sites, sparks, widgets,
};

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
        his::rollup::rollup_his,
        his::flush::flush_his,
        sparks::create::create_spark,
        sparks::list::list_sparks,
        sparks::ack::ack_spark,
        query::run::run_query,
        boards::run::run_board,
        boards::create::create_board,
        boards::list::list_boards,
        boards::get::get_board,
        boards::delete::delete_board,
        boards::run_stored::run_stored_board,
        agent::chat::chat,
        widgets::create::create_widget,
        widgets::list::list_widgets,
        runs::list::list_runs,
        runs::get::get_run,
        runs::resume::resume_run,
        runs::cancel::cancel_run,
    ),
    components(schemas(ErrorBody, RunRecord, RunStatus, RunOrigin, PendingWrite,
        runs::resume::ResumeResponse))
)]
pub struct ApiDoc;

pub(super) async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
