use std::fs;

use argh::FromArgs;
use camino::Utf8PathBuf;
use serde_json::json;
use up_core::jwt::{self, DEFAULT_AUDIENCE, DEFAULT_ISSUER};

use crate::CliError;

/// Verify JSON Web Token signed by a key in a given file.
#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "jwt")]
pub struct VerifyJwt {
    /// path to JWKS file containing signing keys
    #[argh(positional)]
    jwks_file_name: Utf8PathBuf,
    /// path to file containing JWT to verify
    #[argh(positional)]
    jwt_file_name: Utf8PathBuf,
    /// name of issuer to verify (default: up.sector42.io/auth)
    #[argh(option, default = "DEFAULT_ISSUER.to_string()")]
    issuer: String,
    /// name of audience to verify (default: up.sector42.io/server)
    #[argh(option, default = "DEFAULT_AUDIENCE.to_string()")]
    audience: String,
}

impl VerifyJwt {
    pub async fn run(&self) -> Result<(), CliError> {
        tracing::info!(
            "verifying JWT from {} signed by a key in {}",
            self.jwt_file_name,
            self.jwks_file_name
        );

        let jwks = fs::read_to_string(&self.jwks_file_name)?;
        let verifier =
            jwt::Verifier::new_from_jwks(&jwks, Some(&self.issuer), Some(&self.audience))
                .map_err(CliError::JWTVerificationError)?;

        let jwt = fs::read_to_string(&self.jwt_file_name)?;
        let claims = verifier
            .verify(&jwt)
            .map_err(CliError::JWTVerificationError)?;

        tracing::info!("JWT verified with claims: {}", json!(claims).to_string());

        Ok(())
    }
}
