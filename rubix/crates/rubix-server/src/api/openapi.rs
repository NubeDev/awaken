//! OpenAPI document assembly; served at `/api-docs/openapi.json`.

use axum::Json;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::agent::{PendingWrite, RunOrigin, RunRecord, RunStatus};
use crate::auth::{Role, Scope, TokenRecord};
use crate::error::ErrorBody;

use super::{
    agent, boards, command, equips, health, his, points, query, runs, sites, sparks, tokens,
    widgets,
};

/// Registers the `bearer` HTTP security scheme so routes can mark themselves
/// `security(("bearer" = []))`. STACK-DEISGN.md "JWT middleware on axum": OIDC
/// JWTs and PATs are both presented as `Authorization: Bearer …`.
struct BearerSecurity;

impl Modify for BearerSecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&BearerSecurity),
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
        tokens::create::create_token,
        tokens::list::list_tokens,
        tokens::revoke::revoke_token,
    ),
    components(schemas(ErrorBody, RunRecord, RunStatus, RunOrigin, PendingWrite,
        runs::resume::ResumeResponse, TokenRecord, Role, Scope,
        tokens::create::IssueToken, tokens::create::IssuedToken))
)]
pub struct ApiDoc;

pub(super) async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
