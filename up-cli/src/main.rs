use argh::FromArgs;
use dotenv::dotenv;
use thiserror::Error;
use tracing_subscriber::EnvFilter;

mod generate;
mod verify;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("OpenSSL error: {0}")]
    OpenSSLError(#[from] openssl::error::ErrorStack),
    #[error("I/O error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    JSONSerializationError(#[from] serde_json::Error),
    #[error("JWT/JWKS generation failed: {0}")]
    JWTJWKSGenerationError(#[source] up_core::Error),
    #[error("JWT verification failed: {0}")]
    JWTVerificationError(#[source] up_core::Error),
}

/// Command-line interface for UP admin and operations tasks.
#[derive(FromArgs, PartialEq, Eq, Debug)]
pub struct Arguments {
    #[argh(subcommand)]
    command: RootCommand,
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand)]
pub enum RootCommand {
    Generate(generate::GenerateCommand),
    Verify(verify::VerifyCommand),
}

impl RootCommand {
    pub async fn run(&self) -> Result<(), CliError> {
        match self {
            RootCommand::Generate(cmd) => cmd.run().await,
            RootCommand::Verify(cmd) => cmd.run().await,
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    if std::env::var_os("RUST_BACKTRACE").is_none() {
        std::env::set_var("RUST_BACKTRACE", "1")
    }

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "upcli=debug")
    }

    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args: Arguments = argh::from_env();
    if let Err(e) = args.command.run().await {
        tracing::error!("command failed: {:?}", e);
        std::process::exit(1);
    }
}
