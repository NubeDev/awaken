//! `read_point` tool: an agent reads a point's current value by keyexpr.
//! Read-only — no gating beyond tool permission (enforced by the runtime).

use crate::prelude::*;

/// Read the current value of a point.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadPointArgs {
    /// Point keyexpr: `{org}/{site}/{equip-path}/{point}`.
    pub point: String,
}

/// Reads point current values through an injected [`PointAccess`].
pub struct ReadPointTool {
    access: Arc<dyn PointAccess>,
}

impl ReadPointTool {
    pub fn new(access: Arc<dyn PointAccess>) -> Self {
        Self { access }
    }
}

#[async_trait]
impl TypedTool for ReadPointTool {
    type Args = ReadPointArgs;

    fn tool_id(&self) -> &str {
        "rubix_read_point"
    }

    fn name(&self) -> &str {
        "read_point"
    }

    fn description(&self) -> &str {
        "Read the current value of a BMS point by its keyexpr \
         ({org}/{site}/{equip-path}/{point})."
    }

    fn category(&self) -> Option<&str> {
        Some("bms")
    }

    async fn execute(
        &self,
        args: Self::Args,
        _ctx: &ToolCallContext,
    ) -> Result<ToolOutput, ToolError> {
        let value = self
            .access
            .read_point(&args.point)
            .map_err(|e| ToolError::Internal(e.to_string()))?;
        let data = serde_json::json!({ "point": args.point, "value": value });
        Ok(ToolResult::success("read_point", data).into())
    }
}
