use std::fs;

use argh::FromArgs;
use camino::Utf8PathBuf;
use openssl::{
    hash::{Hasher, MessageDigest},
    rsa::Rsa,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

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
        let keypair = Rsa::private_key_from_pem(&pem)?;
        let public_key_der = keypair.public_key_to_der()?;
        let mut hasher = Hasher::new(MessageDigest::sha256())?;
        hasher.update(&public_key_der)?;
        let digest_bytes = hasher.finish()?;

        let n = base64::encode_config(&keypair.n().to_vec(), base64::URL_SAFE_NO_PAD);
        let e = base64::encode_config(&keypair.e().to_vec(), base64::URL_SAFE_NO_PAD);
        let kid = base64::encode_config(&digest_bytes, base64::URL_SAFE_NO_PAD);

        let jwks = JWKS {
            keys: vec![JWK {
                n,
                e,
                kty: KeyType::RSA,
                alg: Some(KeyAlgorithm::RS256),
                kid: Some(kid),
            }],
        };

        let jwks_json = json!(jwks).to_string();

        tracing::info!("saving JWKS to {}", self.file_name);

        fs::write(&self.file_name, jwks_json.as_bytes())?;

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct JWKS {
    keys: Vec<JWK>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
struct JWK {
    kty: KeyType,
    alg: Option<KeyAlgorithm>,
    kid: Option<String>,
    n: String,
    e: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
enum KeyAlgorithm {
    RS256,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
enum KeyType {
    RSA,
}
