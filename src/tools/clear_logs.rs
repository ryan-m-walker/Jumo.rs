use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    events::AppEvent,
    state::AppState,
    tools::{Tool, ToolInput},
};

#[derive(Serialize, Deserialize, JsonSchema)]
pub enum ClearLogsToolLogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ClearLogsToolInputSchema {
    /// Optional log level to clear. If not provided, all logs will be cleared.
    level: Option<ClearLogsToolLogLevel>,
}

pub struct ClearLogsTool;

impl Tool for ClearLogsTool {
    const NAME: &'static str = "clear_logs";

    fn get_tool_input(&self) -> ToolInput {
        ToolInput {
            name: Self::NAME,
            description: "Clears all output logs in the log TUI view.",
            input_schema: schema_for!(ClearLogsToolInputSchema),
        }
    }

    async fn execute(
        &self,
        _input: &str,
        _state: &AppState,
        event_sender: mpsc::Sender<AppEvent>,
    ) -> Result<String, anyhow::Error> {
        event_sender.send(AppEvent::ClearLogs).await?;
        Ok(String::new())
    }
}
