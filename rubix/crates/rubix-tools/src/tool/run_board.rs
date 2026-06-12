//! `run_board` tool: an agent evaluates a reflow control/rule board once.
//!
//! The board is the same JSON the `POST /boards/run` endpoint takes. Board
//! writes go through the priority array, so an agent commanding a point via a
//! board is gated exactly as a direct `write_point` is — the host runs the
//! board over the store-backed access.

use crate::port::BoardAccess;
use crate::prelude::*;

/// Run a reflow board over the BMS points.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunBoardArgs {
    /// The board graph: `nodes` (each with `id`, `component`, `config`) wired by
    /// `connections`. Components: `read_point`, `write_point`, `query_his`.
    pub board: serde_json::Value,
}

/// Runs boards through an injected [`BoardAccess`].
pub struct RunBoardTool {
    access: Arc<dyn BoardAccess>,
}

impl RunBoardTool {
    pub fn new(access: Arc<dyn BoardAccess>) -> Self {
        Self { access }
    }
}

#[async_trait]
impl TypedTool for RunBoardTool {
    type Args = RunBoardArgs;

    fn tool_id(&self) -> &str {
        "rubix_run_board"
    }

    fn name(&self) -> &str {
        "run_board"
    }

    fn description(&self) -> &str {
        "Evaluate a reflow control/rule board once over the BMS points and \
         return each node's outputs. Board writes go through the priority array."
    }

    fn category(&self) -> Option<&str> {
        Some("bms")
    }

    async fn execute(
        &self,
        args: Self::Args,
        _ctx: &ToolCallContext,
    ) -> Result<ToolOutput, ToolError> {
        let outputs = self
            .access
            .run_board(args.board)
            .await
            .map_err(|e| ToolError::Internal(e.to_string()))?;
        Ok(ToolResult::success("run_board", outputs).into())
    }
}
