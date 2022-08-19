use std::fs;

use argh::FromArgs;
use camino::Utf8PathBuf;
use chrono::{
    naive::{serde::ts_seconds, NaiveDateTime},
    {Duration, Utc},
};
use openssl::{hash::MessageDigest, rsa::Rsa, sign::Signer};
use serde::{Deserialize, Serialize};

use crate::{generate::jwks, CliError};

const DEFAULT_ISSUER: &str = "up.sector42.io/auth";
const DEFAULT_AUDIENCE: &str = "up.sector42.io/auth";

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
        let keypair = Rsa::private_key_from_pem(&pem)?;

        let key_id = jwks::compute_key_id(&keypair)?;
        let header = Header {
            key_id,
            algorithm: String::from("RS256"),
        };
        let claims = Claims::new(
            &self.issuer,
            &self.audience,
            &self.subject,
            vec![],
            self.expiry_hours,
        );

        let header_json = serde_json::to_string(&header)?;
        let header_base64 = base64::encode_config(header_json.as_bytes(), base64::URL_SAFE_NO_PAD);
        let claims_json = serde_json::to_string(&claims)?;
        let claims_base64 = base64::encode_config(claims_json.as_bytes(), base64::URL_SAFE_NO_PAD);

        let sign_text = format!("{}.{}", header_base64, claims_base64);

        let private_key = openssl::pkey::PKey::private_key_from_pem(&pem)?;
        let mut signer = Signer::new(MessageDigest::sha256(), private_key.as_ref())?;
        signer.update(sign_text.as_bytes())?;
        let signature = signer.sign_to_vec()?;

        let signature_base64 = base64::encode_config(&signature, base64::URL_SAFE_NO_PAD);
        let jwt = format!("{}.{}", sign_text, signature_base64);

        tracing::info!("saving JWT to {}", self.file_name);

        fs::write(&self.file_name, jwt.as_bytes())?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    #[serde(rename = "kid")]
    pub key_id: String,
    #[serde(rename = "alg")]
    pub algorithm: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    #[serde(skip_serializing_if = "Option::is_none", rename = "iss")]
    pub issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "aud")]
    pub audience: Option<String>,
    #[serde(rename = "iat", with = "ts_seconds")]
    pub issued_at: NaiveDateTime,
    #[serde(rename = "exp", with = "ts_seconds")]
    pub expires_at: NaiveDateTime,
    #[serde(skip_serializing_if = "Option::is_none", rename = "sub")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub roles: Vec<Role>,
}

impl Claims {
    fn new(
        issuer: &str,
        audience: &str,
        subject: &str,
        roles: Vec<Role>,
        expiry_hours: i64,
    ) -> Self {
        let now = Utc::now().naive_utc();
        let hours = Duration::hours(expiry_hours.abs());
        let expires_at = if expiry_hours < 0 {
            now - hours
        } else {
            now + hours
        };

        Self {
            issuer: Some(issuer.to_string()),
            audience: Some(audience.to_string()),
            issued_at: if expiry_hours < 0 { expires_at } else { now },
            expires_at,
            subject: Some(subject.to_string()),
            roles,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Administrator,
    Editor,
    Viewer,
}
