use app::App;
use miette::Result;
use up_server::app;

#[tokio::main]
async fn main() -> Result<()> {
    App::new().run().await
}
