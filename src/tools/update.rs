use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    events::AppEvent,
    state::AppState,
    tools::{Tool, ToolInput},
};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PassToolInputSchema {
    pub production_build: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateToolOutput {
    pub success: bool,
}

pub struct UpdateTool;

impl Tool for UpdateTool {
    const NAME: &'static str = "update";

    fn get_tool_input(&self) -> ToolInput {
        ToolInput {
            name: Self::NAME,
            description: "Auto update yourself by pulling your source code from GitHub and rebuilding the rust binary and then restarting the app.",
            input_schema: schema_for!(PassToolInputSchema),
        }
    }

    async fn execute(
        &self,
        input: &str,
        _state: &AppState,
        _event_sender: mpsc::Sender<AppEvent>,
    ) -> Result<String, anyhow::Error> {
        let _parsed_input: PassToolInputSchema = serde_json::from_str(&input)?;
        let output = UpdateToolOutput { success: true };
        Ok(serde_json::to_string(&output)?)
    }
}
