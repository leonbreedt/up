pub mod auth;
pub mod jwks;
pub mod jwt;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("OpenSSL error: {0}")]
    OpenSSLError(#[from] openssl::error::ErrorStack),
    #[error("I/O error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    JSONSerializationError(#[from] serde_json::Error),
    #[error("JWT validation error: {0}")]
    JWTVerificationError(#[from] alcoholic_jwt::ValidationError),
    #[error("JWT has no key ID, or not found in keys")]
    JWTMissingKid,
}

pub const CA_CERTIFICATE_ENV: &str = "CA_CERTIFICATE";
pub const SERVER_CERTIFICATE_ENV: &str = "SERVER_CERTIFICATE";
pub const JWKS_ENV: &str = "JWKS";
