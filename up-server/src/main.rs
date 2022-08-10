use miette::Result;

mod api;
mod app;
mod database;
mod integrations;
mod jobs;
mod mask;
mod notifier;
mod repository;
mod shortid;

#[tokio::main]
async fn main() -> Result<()> {
    app::App::new().run().await
}
