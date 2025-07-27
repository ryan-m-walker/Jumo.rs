use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    events::AppEvent,
    state::{AppState, View},
    tools::{Tool, ToolInput},
};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SetViewToolInputSchema {
    pub view: View,
}

#[derive(Serialize, Deserialize)]
pub struct SetViewToolOutput {
    pub previous_view: View,
    pub new_view: View,
}

pub struct SetViewTool;

impl Tool for SetViewTool {
    const NAME: &'static str = "set_view";

    fn get_tool_input(&self) -> ToolInput {
        ToolInput {
            name: Self::NAME,
            description: "Set the current view for your TUI display.",
            input_schema: schema_for!(SetViewToolInputSchema),
        }
    }

    async fn execute(
        &self,
        input: &str,
        state: &AppState,
        event_sender: mpsc::Sender<AppEvent>,
    ) -> Result<String, anyhow::Error> {
        let parsed_input: SetViewToolInputSchema = serde_json::from_str(input)?;

        let output = SetViewToolOutput {
            previous_view: state.view,
            new_view: parsed_input.view,
        };

        event_sender
            .send(AppEvent::SetView(parsed_input.view))
            .await?;

        Ok(serde_json::to_string(&output)?)
    }
}
