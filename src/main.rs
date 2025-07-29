use std::panic;

use colored::Colorize;

use crate::app::App;

mod app;
mod audio;
mod camera;
mod database;
mod emote;
mod environment;
mod events;
mod prompts;
mod services;
mod state;
mod text_processor;
mod tools;
mod widgets;

#[tokio::main]
async fn main() {
    let terminal = ratatui::init();

    dotenv::dotenv().ok();

    panic::set_hook(Box::new(|info| {
        ratatui::restore();
        eprintln!("{info}");
    }));

    let mut app = App::new(terminal);

    if let Err(err) = app.start().await {
        ratatui::restore();

        let err_message = format!("[Error]: {err}").red();
        eprintln!("{err_message}");

        return;
    }

    ratatui::restore();
}
