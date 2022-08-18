use argh::FromArgs;

use crate::CliError;

mod ca_certificate;
mod keypair;

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
}

impl GenerateCommand {
    pub async fn run(&self) -> Result<(), CliError> {
        match &self.subcommand {
            GenerateSubCommand::Keypair(cmd) => cmd.run().await,
            GenerateSubCommand::CACertificate(cmd) => cmd.run().await,
        }
    }
}
