use std::fs;

use argh::FromArgs;
use camino::Utf8PathBuf;
use up_core::jwt;

use crate::CliError;

pub const DEFAULT_ISSUER: &str = "up.sector42.io/auth";
pub const DEFAULT_AUDIENCE: &str = "up.sector42.io/auth";

/// Issue JSON Web Token signed by a key in a given file.
#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "jwt")]
pub struct GenerateJwt {
    /// path to PEM file containing signing key
    #[argh(positional)]
    key_file_name: Utf8PathBuf,
    /// path to output JWT file
    #[argh(positional)]
    file_name: Utf8PathBuf,
    /// subject the JWT is issued for, e.g. anonymized user ID.
    #[argh(positional)]
    subject: String,
    /// name of issuer (default: up.sector42.io/auth)
    #[argh(option, default = "DEFAULT_ISSUER.to_string()")]
    issuer: String,
    /// name of audience (default: up.sector42.io/server)
    #[argh(option, default = "DEFAULT_AUDIENCE.to_string()")]
    audience: String,
    /// how long until the JWT expires, in hours from now (default: 12)
    #[argh(option, default = "12")]
    expiry_hours: i64,
}

impl GenerateJwt {
    pub async fn run(&self) -> Result<(), CliError> {
        tracing::info!("issuing JWT signed by key in {}", self.key_file_name,);

        let pem = fs::read(&self.key_file_name)?;
        let generator = jwt::Generator::new_from_pem(&pem, &self.issuer, &self.audience)
            .map_err(CliError::JWTJWKSGenerationError)?;

        let jwt = generator
            .generate(&self.subject, self.expiry_hours, None)
            .map_err(CliError::JWTJWKSGenerationError)?;

        tracing::info!("saving JWT to {}", self.file_name);
        fs::write(&self.file_name, jwt.as_bytes())?;

        Ok(())
    }
}
