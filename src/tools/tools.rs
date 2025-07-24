use tokio::sync::mpsc;

use crate::{
    events::AppEvent,
    state::AppState,
    tools::{Tool, ToolInput, pass::PassTool, set_view::SetViewTool, update::UpdateTool},
};

pub enum ToolType {
    Pass(PassTool),
    Update(UpdateTool),
    SetView(SetViewTool),
}

impl ToolType {
    pub fn to_tool_input(&self) -> ToolInput {
        match self {
            ToolType::Pass(tool) => tool.get_tool_input(),
            ToolType::Update(tool) => tool.get_tool_input(),
            ToolType::SetView(tool) => tool.get_tool_input(),
        }
    }

    pub fn all_tools() -> Vec<ToolInput> {
        vec![
            Self::Pass(PassTool).to_tool_input(),
            Self::Update(UpdateTool).to_tool_input(),
            Self::SetView(SetViewTool).to_tool_input(),
        ]
    }

    pub async fn execute_tool(
        tool_name: &str,
        input: &str,
        event_sender: mpsc::Sender<AppEvent>,
    ) -> Result<String, anyhow::Error> {
        match tool_name {
            PassTool::NAME => PassTool.execute(input, event_sender).await,
            UpdateTool::NAME => UpdateTool.execute(input, event_sender).await,
            SetViewTool::NAME => SetViewTool.execute(input, event_sender).await,
            _ => Err(anyhow::anyhow!("Tool not found")),
        }
    }
}
