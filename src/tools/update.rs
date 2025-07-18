pub struct UpdateTool {
    name: String,
    description: String,
    input_schema: String,
}

impl UpdateTool {
    pub fn new() -> Self {
        Self {
            name: "self_update".to_string(),
            description: "Auto update yourself by pulling your source code from GitHub and rebuilding the rust binary and then restarting the app.".to_string(),
            input_schema: r#"
            {
                "type": "object",
                "properties": {
                    "version": {
                        "type": "string",
                        "description": "The version of the app to update to"
                    }
                },
                "required": ["version"]
            }
            "#
            .to_string(),
        }
    }
}
