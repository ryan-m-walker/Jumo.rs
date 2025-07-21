use crate::app::App;

mod app;
mod audio;
mod database;
mod events;
mod prompts;
mod services;
mod state;
mod text_processor;
mod tools;
mod widgets;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let terminal = ratatui::init();

    let mut app = App::new(terminal);
    if let Err(err) = app.start().await {
        eprintln!("{err}");
    }

    ratatui::restore();
}
