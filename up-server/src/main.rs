use miette::Result;

mod api;
mod app;
mod database;
mod shortid;
mod repository;

#[tokio::main]
async fn main() -> Result<()> {
    app::App::new().run().await
}
