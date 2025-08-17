use crate::{
    prompts::{
        emoting::get_emote_prompt, overview::get_overview_prompt, status::get_status_prompt,
        system_info::get_system_info_prompt,
    },
    state::AppState,
};

pub mod emoting;
pub mod overview;
pub mod status;
pub mod system_info;

pub fn get_system_prompt(state: &AppState) -> String {
    [
        get_overview_prompt(),
        get_status_prompt(state),
        get_system_info_prompt(),
        get_emote_prompt(state),
    ]
    .join("\n\n")
}
