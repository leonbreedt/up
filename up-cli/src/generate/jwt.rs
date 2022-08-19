use std::fs;

use argh::FromArgs;
use camino::Utf8PathBuf;
use up_core::jwt::{self, DEFAULT_AUDIENCE, DEFAULT_ISSUER};
use up_core::SERVER_CERTIFICATE_ENV;

use crate::generate::env_or_error;
use crate::CliError;

/// Issue JSON Web Token signed by a key in a given file.
#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "jwt")]
pub struct GenerateJwt {
    /// path to PEM file containing signing key
    #[argh(option)]
    key_file: Option<Utf8PathBuf>,
    /// path to output JWT file
    #[argh(option)]
    jwt_file: Option<Utf8PathBuf>,
    /// subject the JWT is issued for, e.g. anonymized user ID.
    #[argh(option)]
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
        let pem = if let Some(key_file) = &self.key_file {
            let pem = fs::read(key_file)?;
            tracing::info!("issuing JWT signed by key in {}", key_file);
            pem
        } else {
            let pem = env_or_error(SERVER_CERTIFICATE_ENV, "JWT generation")?
                .as_bytes()
                .to_vec();
            tracing::info!(
                "issuing JWT signed by key from environment variable {}",
                SERVER_CERTIFICATE_ENV
            );
            pem
        };

        let generator = jwt::Generator::new_from_pem(&pem, &self.issuer, &self.audience)
            .map_err(CliError::JWTJWKSGenerationError)?;

        let jwt = generator
            .generate(&self.subject, self.expiry_hours, None)
            .map_err(CliError::JWTJWKSGenerationError)?;

        tracing::info!("generated JWT signed by key ID {}", generator.key_id());

        if let Some(jwt_file) = &self.jwt_file {
            tracing::info!("saving JWT to {}", jwt_file);
            fs::write(jwt_file, jwt.as_bytes())?;
        } else {
            println!("{}", jwt);
        }

        Ok(())
    }
}
