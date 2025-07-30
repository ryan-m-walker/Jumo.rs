use std::panic;

use colored::Colorize;

use crate::{app::App, camera::Camera};

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
    let result = run().await;

    ratatui::restore();

    if let Err(e) = result {
        eprintln!("{}", format!("[Error]: {e}").red());
    }
}

async fn run() -> Result<(), anyhow::Error> {
    panic::set_hook(Box::new(|e| {
        ratatui::restore();
        eprintln!("{}", format!("[Error]: {e}").red());
    }));

    Camera::start_nokhwa()?;
    dotenv::dotenv()?;

    let terminal = ratatui::init();

    let mut app = App::new(terminal);
    app.start().await?;

    Ok(())
}
