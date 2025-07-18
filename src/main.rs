use crate::app::App;

mod app;
mod audio;
mod events;
mod renderer;
mod services;
mod state;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let mut app = App::new();
    if let Err(err) = app.start().await {
        eprintln!("{err}");
    }

    ratatui::restore();
}
