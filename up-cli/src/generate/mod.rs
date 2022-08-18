use argh::FromArgs;

use crate::CliError;

mod keypair;

/// Generates keys and certificates.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "generate")]
pub struct GenerateCommand {
    #[argh(subcommand)]
    subcommand: GenerateSubCommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum GenerateSubCommand {
    Keypair(keypair::GenerateKeypairCommand),
}

impl GenerateCommand {
    pub async fn run(&self) -> Result<(), CliError> {
        match &self.subcommand {
            GenerateSubCommand::Keypair(cmd) => cmd.run().await,
        }
    }
}
