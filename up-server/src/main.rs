use miette::Result;

mod api;
mod app;
mod database;
mod repository;

#[tokio::main]
async fn main() -> Result<()> {
    app::App::new().run().await
}
