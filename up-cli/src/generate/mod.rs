use argh::FromArgs;

use crate::CliError;

pub mod ca_certificate;
pub mod certificate;
pub mod jwks;
pub mod jwt;
pub mod keypair;
pub mod server_env;

/// Generates keys and certificates.
#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "generate")]
pub struct GenerateCommand {
    #[argh(subcommand)]
    subcommand: GenerateSubCommand,
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand)]
pub enum GenerateSubCommand {
    Keypair(keypair::GenerateKeypairCommand),
    CACertificate(ca_certificate::GenerateCACertificateCommand),
    Certificate(certificate::GenerateCertificateCommand),
    Jwks(jwks::GenerateJwks),
    Jwt(jwt::GenerateJwt),
    ServerEnv(server_env::GenerateServerEnv),
}

impl GenerateCommand {
    pub async fn run(&self) -> Result<(), CliError> {
        match &self.subcommand {
            GenerateSubCommand::Keypair(cmd) => cmd.run().await,
            GenerateSubCommand::CACertificate(cmd) => cmd.run().await,
            GenerateSubCommand::Certificate(cmd) => cmd.run().await,
            GenerateSubCommand::Jwks(cmd) => cmd.run().await,
            GenerateSubCommand::Jwt(cmd) => cmd.run().await,
            GenerateSubCommand::ServerEnv(cmd) => cmd.run().await,
        }
    }
}

fn env_or_error(name: &str, purpose: &str) -> Result<String, CliError> {
    if let Ok(value) = std::env::var(name) {
        Ok(value)
    } else {
        Err(CliError::MissingEnvironmentVariable {
            name: name.to_string(),
            purpose: purpose.to_string(),
        })
    }
}
