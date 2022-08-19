use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::str::FromStr;

use crate::{jwt, Error};

#[derive(Clone, Serialize, Deserialize)]
pub struct Jwks {
    keys: Vec<Jwk>,
}

impl Jwks {
    pub fn from_pem(pem: &[u8]) -> Result<Self, Error> {
        let private_key = Rsa::private_key_from_pem(pem)?;
        let public_key = PKey::public_key_from_pem(pem)?;

        let n = base64::encode_config(&private_key.n().to_vec(), base64::URL_SAFE_NO_PAD);
        let e = base64::encode_config(&private_key.e().to_vec(), base64::URL_SAFE_NO_PAD);
        let kid = jwt::compute_key_id(&public_key)?;

        Ok(Self {
            keys: vec![Jwk {
                n,
                e,
                kty: KeyType::Rsa,
                alg: Some(KeyAlgorithm::Rs256),
                kid: Some(kid),
            }],
        })
    }

    pub fn key_ids(&self) -> Vec<&str> {
        self.keys
            .iter()
            .map(|k| k.kid.as_deref().unwrap())
            .collect()
    }
}

impl ToString for Jwks {
    fn to_string(&self) -> String {
        json!(self).to_string()
    }
}

impl FromStr for Jwks {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str(s)?)
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Jwk {
    kty: KeyType,
    alg: Option<KeyAlgorithm>,
    kid: Option<String>,
    n: String,
    e: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum KeyAlgorithm {
    Rs256,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum KeyType {
    Rsa,
}
