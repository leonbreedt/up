use miette::Result;

mod api;
mod app;
mod database;
mod mask;
mod repository;
mod shortid;

#[tokio::main]
async fn main() -> Result<()> {
    app::App::new().run().await
}
