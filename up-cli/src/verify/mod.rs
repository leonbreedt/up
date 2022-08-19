use argh::FromArgs;

use crate::CliError;

mod jwt;

/// Verifies tokens and signatures.
#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "verify")]
pub struct VerifyCommand {
    #[argh(subcommand)]
    subcommand: VerifySubCommand,
}

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand)]
pub enum VerifySubCommand {
    Jwt(jwt::VerifyJwt),
}

impl VerifyCommand {
    pub async fn run(&self) -> Result<(), CliError> {
        match &self.subcommand {
            VerifySubCommand::Jwt(cmd) => cmd.run().await,
        }
    }
}
