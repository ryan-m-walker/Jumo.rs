use crate::app::App;

mod app;
mod audio;
mod events;
mod renderer;
mod services;
mod state;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();
    let mut app = App::new();
    app.start().await?;
    Ok(())
}
