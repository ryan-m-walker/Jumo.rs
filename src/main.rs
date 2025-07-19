use rusqlite::Connection;

use crate::app::App;

mod app;
mod audio;
mod database;
mod events;
mod prompts;
mod services;
mod state;
mod tools;
mod widgets;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let mut app = App::new();
    if let Err(err) = app.start().await {
        eprintln!("{err}");
    }

    ratatui::restore();
}
