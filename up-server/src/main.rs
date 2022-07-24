mod api;
mod app;
mod database;
mod repository;

#[tokio::main]
async fn main() {
    app::App::new().run().await;
}
