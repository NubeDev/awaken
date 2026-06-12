//! `pin_widget` tool: an agent pins a dashboard tile so a finding or trend it
//! surfaced during a turn persists on the site dashboard.
//!
//! Read/write of points has its own gating; pinning a widget is a UI-state
//! write with no priority-array effect, so the only checks are argument shape
//! and that the owning site exists (enforced by the host).

use std::sync::Arc;

use uuid::Uuid;

use crate::port::WidgetAccess;
use crate::prelude::*;

/// Pin a widget on a site dashboard.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PinWidgetArgs {
    /// Site UUID the widget belongs to.
    pub site_id: String,
    /// Widget kind: `point_value`, `point_history`, or `board_output`.
    pub kind: String,
    /// Human-facing tile title.
    pub title: String,
    /// What the tile points at: a point keyexpr (`point_*` kinds) or a board
    /// slug (`board_output`).
    pub target: String,
}

/// Valid widget kinds, matching `rubix_core::WidgetKind`'s serde tokens.
const KINDS: [&str; 3] = ["point_value", "point_history", "board_output"];

/// Pins dashboard widgets through an injected [`WidgetAccess`].
pub struct PinWidgetTool {
    access: Arc<dyn WidgetAccess>,
}

impl PinWidgetTool {
    pub fn new(access: Arc<dyn WidgetAccess>) -> Self {
        Self { access }
    }
}

#[async_trait]
impl TypedTool for PinWidgetTool {
    type Args = PinWidgetArgs;

    fn tool_id(&self) -> &str {
        "rubix_pin_widget"
    }

    fn name(&self) -> &str {
        "pin_widget"
    }

    fn description(&self) -> &str {
        "Pin a dashboard widget on a site so a finding or trend persists for \
         operators. kind is point_value, point_history, or board_output; \
         target is a point keyexpr or board slug."
    }

    fn category(&self) -> Option<&str> {
        Some("bms")
    }

    async fn execute(
        &self,
        args: Self::Args,
        _ctx: &ToolCallContext,
    ) -> Result<ToolOutput, ToolError> {
        let site_id = Uuid::parse_str(args.site_id.trim())
            .map_err(|_| ToolError::InvalidArguments(format!("invalid site_id: {}", args.site_id)))?;
        let kind = args.kind.trim();
        if !KINDS.contains(&kind) {
            return Err(ToolError::InvalidArguments(format!(
                "kind must be one of {KINDS:?}, got {kind:?}"
            )));
        }
        if args.title.trim().is_empty() {
            return Err(ToolError::InvalidArguments("title must not be empty".into()));
        }
        if args.target.trim().is_empty() {
            return Err(ToolError::InvalidArguments("target must not be empty".into()));
        }
        let id = self
            .access
            .pin_widget(site_id, kind, args.title.trim(), args.target.trim())
            .await
            .map_err(|e| ToolError::Internal(e.to_string()))?;
        let data = serde_json::json!({
            "widget_id": id,
            "site_id": site_id,
            "kind": kind,
            "title": args.title.trim(),
            "target": args.target.trim(),
        });
        Ok(ToolResult::success("pin_widget", data).into())
    }
}
