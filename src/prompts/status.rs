use crate::state::AppState;

pub fn get_status_prompt(state: &AppState) -> String {
    let date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let view = state.view;

    format!(
        r#"
## Status:

### General:

- Current date and time is {date}.

### TUI state:

- Current view is {view}.
"#,
    )
}
