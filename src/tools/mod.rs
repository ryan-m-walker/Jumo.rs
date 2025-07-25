use schemars::Schema;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{events::AppEvent, state::AppState};

pub mod clear_logs;
pub mod pass;
pub mod set_view;
pub mod tools;
pub mod update;

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInput {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Schema,
}

pub trait Tool {
    const NAME: &'static str;
    fn get_tool_input(&self) -> ToolInput;
    async fn execute(
        &self,
        input: &str,
        state: &AppState,
        event_sender: mpsc::Sender<AppEvent>,
    ) -> Result<String, anyhow::Error>;
}
