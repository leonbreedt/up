use std::fs;

use argh::FromArgs;
use camino::Utf8PathBuf;
use up_core::jwks::Jwks;

use crate::CliError;

/// Generate JSON Web Key Set for a given key.
#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "jwks")]
pub struct GenerateJwks {
    // path to PEM file containing signing key
    #[argh(positional)]
    key_file_name: Utf8PathBuf,
    // path to output JWKS file
    #[argh(positional)]
    file_name: Utf8PathBuf,
}

impl GenerateJwks {
    pub async fn run(&self) -> Result<(), CliError> {
        tracing::info!("generating JWKS from key in {}", self.key_file_name,);

        let pem = fs::read(&self.key_file_name)?;

        let jwks = Jwks::from_pem(&pem)
            .map_err(CliError::JWTJWKSGenerationError)?
            .to_string();

        tracing::info!("saving JWKS to {}", self.file_name);

        fs::write(&self.file_name, jwks.as_bytes())?;

        Ok(())
    }
}
