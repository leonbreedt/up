use std::str::FromStr;

use harsh::Harsh;
use lazy_static::lazy_static;
use miette::Diagnostic;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use uuid::Uuid;

lazy_static! {
    static ref HARSH: Harsh = Harsh::builder().salt("io.sector42.up").build().unwrap();
}

#[derive(Debug, Clone, Copy)]
pub struct ShortId(Uuid);

impl ShortId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: &Uuid) -> Self {
        id.into()
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for ShortId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for ShortId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<&Uuid> for ShortId {
    fn from(id: &Uuid) -> Self {
        Self(id.clone())
    }
}

impl From<ShortId> for Uuid {
    fn from(id: ShortId) -> Self {
        id.0
    }
}

impl From<&ShortId> for Uuid {
    fn from(id: &ShortId) -> Self {
        id.0
    }
}

impl From<ShortId> for sea_query::Value {
    fn from(id: ShortId) -> Self {
        id.to_string().into()
    }
}

impl FromStr for ShortId {
    type Err = ParseShortIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let decoded = HARSH
            .decode(s)
            .map_err(|_| ParseShortIdError::DecodeFailure)?;
        Ok(Self(Uuid::from_u128(
            decoded[0] as u128 | ((decoded[1] as u128) << 64),
        )))
    }
}

impl ToString for ShortId {
    fn to_string(&self) -> String {
        let n = self.0.as_u128();
        let hi = (n >> 64) as u64;
        let lo = n as u64;
        HARSH.encode(&[lo, hi])
    }
}

impl PartialEq for ShortId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum ParseShortIdError {
    #[error("not a valid identifier")]
    #[diagnostic(code(up::error::bad_argument))]
    DecodeFailure,
}

impl Serialize for ShortId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ShortId {
    fn deserialize<D>(deserializer: D) -> Result<ShortId, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn shortid_string_roundtrip() {
        let id: ShortId = Uuid::from_str("5a3b3743-4f32-4fb6-8d0c-03bc793ff79d")
            .unwrap()
            .into();

        let value = id.to_string();

        assert_eq!("574wP6790o0E5hVakwJbk2mX7L", value);

        let parsed_id: ShortId = value.parse().unwrap();

        assert_eq!(id, parsed_id);
    }

    #[test]
    pub fn malformed_id_does_not_parse() {
        assert!(matches!(
            "574wP6790o0E5hVakwJbk2m7L".parse::<ShortId>(),
            Err(ParseShortIdError::DecodeFailure)
        ))
    }

    #[test]
    pub fn shortid_json_roundtrip() {
        let id: ShortId = Uuid::from_str("45c544f4-36ac-4428-8f9e-187037a6c87b")
            .unwrap()
            .into();

        assert_eq!(
            "\"Xw8QKAZdNA3nnFKxopJ88kweGE\"",
            serde_json::to_string(&id).unwrap()
        );
        assert_eq!(
            id,
            serde_json::from_str("\"Xw8QKAZdNA3nnFKxopJ88kweGE\"").unwrap()
        );
    }
}
