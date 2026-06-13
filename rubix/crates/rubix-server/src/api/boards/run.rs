//! POST /api/v1/boards/run — evaluate a reflow board once over the store.
//!
//! The request carries an inline [`BoardGraph`]; the server loads it into a
//! reflow `Network` backed by [`StorePointAccess`], ticks the source nodes, and
//! returns every node's outport output. Boards read and command real points
//! through the priority array — the same write path as HTTP and the bus.

use axum::extract::State;
use axum::Json;
use rubix_flow::{BoardGraph, NodeOutput};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::error::{ApiError, ErrorBody};
use crate::flow::StorePointAccess;
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RunBoardRequest {
    /// The board graph to evaluate: nodes (`read_point`, `write_point`,
    /// `query_his`) wired by connections.
    #[schema(value_type = Object)]
    pub board: BoardGraph,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RunBoardResponse {
    /// Every outport packet produced this run, in node-declaration order.
    #[schema(value_type = Object)]
    pub outputs: Vec<NodeOutput>,
}

#[utoipa::path(post, path = "/api/v1/boards/run", tag = "boards",
    request_body = RunBoardRequest,
    responses(
        (status = 200, body = RunBoardResponse),
        (status = 400, body = ErrorBody)))]
pub(crate) async fn run_board(
    State(state): State<AppState>,
    Json(req): Json<RunBoardRequest>,
) -> Result<Json<RunBoardResponse>, ApiError> {
    let access = Arc::new(
        StorePointAccess::with_bus(state.store.clone(), state.bus.clone())
            .with_agent(state.agent.clone())
            .with_org(req.board.tenant_org()),
    );
    let outputs = req
        .board
        .run(access)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(RunBoardResponse { outputs }))
}
