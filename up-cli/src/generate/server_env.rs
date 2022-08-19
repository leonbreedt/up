use std::fs;

use argh::FromArgs;
use camino::Utf8PathBuf;
use up_core::jwks;

use crate::{
    generate::{ca_certificate, certificate},
    CliError,
};

const CA_COMMON_NAME: &str = "ca.up.sector42.io";
const COMMON_NAME: &str = "up.sector42.io";

/// Generate a .env file containing server CA, certificate and JWKS environment variables.
#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "server-env")]
pub struct GenerateServerEnv {
    /// path to env file to create (default: server.env in current directory)
    #[argh(positional, default = "Utf8PathBuf::from(\"server.env\")")]
    file_name: Utf8PathBuf,
}

impl GenerateServerEnv {
    pub async fn run(&self) -> Result<(), CliError> {
        tracing::info!("generating new CA certificate and key");
        let ca_certificate_bundle = ca_certificate::generate_ca_certificate_bundle_with_key(
            ca_certificate::DEFAULT_KEY_SIZE,
            CA_COMMON_NAME,
            ca_certificate::DEFAULT_EXPIRY_DAYS,
            None,
        )?;

        tracing::info!("generating new server certificate and key");
        let certificate_bundle = certificate::generate_certificate(
            &ca_certificate_bundle,
            certificate::DEFAULT_KEY_SIZE,
            COMMON_NAME,
            None,
            certificate::DEFAULT_EXPIRY_DAYS,
            None,
        )?;

        tracing::info!("generating JSON Web Key Set for server key");

        let jwks =
            jwks::Jwks::from_pem(&certificate_bundle).map_err(CliError::JWTJWKSGenerationError)?;

        let mut dot_env = String::new();
        dot_env.push_str(&env_line(
            "CA_CERTIFICATE",
            std::str::from_utf8(&ca_certificate_bundle).unwrap(),
        ));
        dot_env.push_str(&env_line(
            "SERVER_CERTIFICATE",
            std::str::from_utf8(&ca_certificate_bundle).unwrap(),
        ));
        dot_env.push_str(&env_line("JWKS", &jwks.to_string()));

        tracing::info!("saving to {}", self.file_name);

        fs::write(&self.file_name, dot_env)?;

        Ok(())
    }
}

fn env_line(name: &str, value: &str) -> String {
    let mut line = String::new();
    line.push_str(name);
    line.push('=');
    line.push_str(&shell_quote(value));
    line.push('\n');
    line
}

fn shell_quote(value: &str) -> String {
    if value.contains('\n') || value.contains('\t') || value.contains('\r') {
        // double quote
        format!(
            "\"{}\"",
            value
                .replace('\n', "\\n")
                .replace('\t', "\\t")
                .replace('\r', "\\r")
        )
    } else {
        // single quote
        format!("'{}'", value)
    }
}
