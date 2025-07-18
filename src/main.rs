use crate::app::App;

mod app;
mod recorder;
mod claude_types;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();
    let mut app = App::new();
    app.start().await?;
    Ok(())
}
