use alcoholic_jwt::Validation;
use chrono::{
    naive::{serde::ts_seconds, NaiveDateTime},
    {Duration, Utc},
};
use openssl::hash::Hasher;
use openssl::pkey::{PKey, Private, Public};
use openssl::{hash::MessageDigest, sign::Signer};
use serde::{Deserialize, Serialize};

use crate::jwks::Jwks;
use crate::{auth::Role, Error};

pub const DEFAULT_ISSUER: &str = "up.sector42.io/auth";
pub const DEFAULT_AUDIENCE: &str = "up.sector42.io/server";

pub struct Generator {
    private_key: PKey<Private>,
    key_id: String,
    issuer: String,
    audience: String,
}

impl Generator {
    pub fn new_from_pem(pem: &[u8], issuer: &str, audience: &str) -> Result<Self, Error> {
        let private_key = PKey::private_key_from_pem(pem)?;
        let public_key = PKey::public_key_from_pem(pem)?;
        let key_id = compute_key_id(&public_key)?;
        Ok(Self {
            private_key,
            key_id,
            issuer: issuer.to_string(),
            audience: audience.to_owned(),
        })
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn generate(
        &self,
        subject: &str,
        expiry_hours: i64,
        roles: Option<Vec<Role>>,
    ) -> Result<String, Error> {
        let header = Header {
            key_id: self.key_id.clone(),
            algorithm: String::from("RS256"),
        };
        let claims = Claims::new(
            &self.issuer,
            &self.audience,
            subject,
            roles.as_deref(),
            expiry_hours,
        );

        let header_json = serde_json::to_string(&header)?;
        let header_base64 = base64::encode_config(header_json.as_bytes(), base64::URL_SAFE_NO_PAD);
        let claims_json = serde_json::to_string(&claims)?;
        let claims_base64 = base64::encode_config(claims_json.as_bytes(), base64::URL_SAFE_NO_PAD);

        let sign_text = format!("{}.{}", header_base64, claims_base64);

        let mut signer = Signer::new(MessageDigest::sha256(), self.private_key.as_ref())?;
        signer.update(sign_text.as_bytes())?;
        let signature = signer.sign_to_vec()?;

        let signature_base64 = base64::encode_config(&signature, base64::URL_SAFE_NO_PAD);
        let jwt = format!("{}.{}", sign_text, signature_base64);

        Ok(jwt)
    }
}

pub struct Verifier {
    jwks: alcoholic_jwt::JWKS,
    internal_jwks: Jwks,
    issuer: Option<String>,
    audience: Option<String>,
}

impl Verifier {
    pub fn new_from_jwks(
        jwks: &str,
        issuer: Option<&str>,
        audience: Option<&str>,
    ) -> Result<Self, Error> {
        let internal_jwks: Jwks = serde_json::from_str(jwks)?;
        Ok(Self {
            jwks: serde_json::from_str(jwks)?,
            internal_jwks,
            issuer: issuer.map(|i| i.to_string()),
            audience: audience.map(|i| i.to_string()),
        })
    }

    pub fn key_ids(&self) -> Vec<&str> {
        self.internal_jwks.key_ids()
    }

    pub fn verify(&self, jwt: &str) -> Result<Claims, Error> {
        if let Some(kid) = alcoholic_jwt::token_kid(jwt)? {
            let jwk = match self.jwks.find(&kid) {
                Some(jwk) => jwk,
                None => return Err(Error::JWTMissingKid),
            };

            let mut validations = vec![Validation::SubjectPresent, Validation::NotExpired];
            if let Some(issuer) = &self.issuer {
                validations.push(Validation::Issuer(issuer.to_owned()));
            }
            if let Some(audience) = &self.audience {
                validations.push(Validation::Audience(audience.to_owned()));
            }

            let valid_jwt = alcoholic_jwt::validate(jwt, jwk, validations)?;

            Ok(serde_json::from_value(valid_jwt.claims)?)
        } else {
            Err(Error::JWTMissingKid)
        }
    }
}

pub fn compute_key_id(public_key: &PKey<Public>) -> Result<String, Error> {
    let public_key_der = public_key.public_key_to_der()?;
    let mut hasher = Hasher::new(MessageDigest::sha256())?;
    hasher.update(&public_key_der)?;
    let digest_bytes = hasher.finish()?;
    let kid = base64::encode_config(&digest_bytes, base64::URL_SAFE_NO_PAD);
    Ok(kid)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Header {
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
        roles: Option<&[Role]>,
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
            roles: roles.map(|r| r.to_vec()).unwrap_or_else(Vec::new),
        }
    }
}
