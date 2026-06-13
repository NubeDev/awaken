//! OpenAPI document assembly; served at `/api-docs/openapi.json`.

use axum::Json;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::agent::{PendingWrite, RunOrigin, RunRecord, RunStatus};
use crate::auth::{Role, Scope, TokenRecord};
use crate::error::ErrorBody;

use super::{
    agent, boards, command, dashboards, datasources, equips, health, his, orgs, points, query,
    rules, runs, sites, sparks, tokens, widgets,
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
        sites::patch::patch_site,
        sites::delete::delete_site,
        orgs::list::list_orgs,
        orgs::create::provision_org,
        equips::create::create_equip,
        equips::list::list_equips,
        equips::get::get_equip,
        equips::patch::patch_equip,
        equips::delete::delete_equip,
        points::create::create_point,
        points::list::list_points,
        points::get::get_point,
        points::patch::patch_point,
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
        sparks::get::get_spark,
        sparks::delete::delete_spark,
        sparks::ack::ack_spark,
        query::run::run_query,
        datasources::run::run_query,
        datasources::named::invoke_named,
        datasources::describe::describe_datasource,
        boards::run::run_board,
        boards::create::create_board,
        boards::list::list_boards,
        boards::get::get_board,
        boards::patch::patch_board,
        boards::delete::delete_board,
        boards::run_stored::run_stored_board,
        boards::components::list_components,
        boards::outputs::board_outputs,
        rules::create::create_rule,
        rules::list::list_rules,
        rules::get::get_rule,
        rules::update::update_rule,
        rules::delete::delete_rule,
        rules::referencing::referencing_rules,
        agent::chat::chat,
        widgets::create::create_widget,
        widgets::list::list_widgets,
        widgets::get::get_widget,
        widgets::patch::patch_widget,
        widgets::delete::delete_widget,
        dashboards::create::create_dashboard,
        dashboards::list::list_dashboards,
        dashboards::get::get_dashboard,
        dashboards::patch::patch_dashboard,
        dashboards::delete::delete_dashboard,
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
        tokens::create::IssueToken, tokens::create::IssuedToken,
        boards::components::ComponentView, boards::components::PortView,
        boards::components::ConfigFieldView, crate::scheduler::PortOutput,
        orgs::list::OrgSummary, orgs::create::ProvisionOrg,
        rules::dto::CreateRule, rules::dto::UpdateRule, rules::dto::RuleView,
        datasources::run::DatasourceQueryRequest, datasources::run::DatasourceResultBody,
        datasources::named::NamedQueryRequest,
        dashboards::create::CreateDashboard, dashboards::patch::PatchDashboard,
        widgets::patch::PatchWidget))
)]
pub struct ApiDoc;

pub(super) async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
