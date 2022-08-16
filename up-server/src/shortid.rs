use std::str::FromStr;

use miette::Diagnostic;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use ulid::Ulid;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct ShortId(Ulid, Uuid);

impl ShortId {
    pub fn new() -> Self {
        let uuid: Uuid = Uuid::new_v4();
        let id: Ulid = uuid.into();
        Self(id, uuid)
    }

    pub fn from_uuid(id: &Uuid) -> Self {
        Self(id.as_u128().into(), *id)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.1
    }

    pub fn into_uuid(self) -> Uuid {
        self.1
    }
}

impl Default for ShortId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for ShortId {
    fn from(id: Uuid) -> Self {
        Self::from_uuid(&id)
    }
}

impl From<&Uuid> for ShortId {
    fn from(id: &Uuid) -> Self {
        Self::from_uuid(id)
    }
}

impl From<ShortId> for Uuid {
    fn from(id: ShortId) -> Self {
        id.1
    }
}

impl From<&ShortId> for Uuid {
    fn from(id: &ShortId) -> Self {
        id.1
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
        let ulid: Ulid = s.parse().map_err(|_| ParseShortIdError::DecodeFailure)?;
        Ok(Self(ulid, ulid.into()))
    }
}

impl ToString for ShortId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl PartialEq for ShortId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum ParseShortIdError {
    #[error("value could not be parsed as an identifier")]
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

        assert_eq!("2T7CVM6KSJ9YV8T303QHWKZXWX", value);

        let parsed_id: ShortId = value.parse().unwrap();

        assert_eq!(id, parsed_id);
    }

    #[test]
    pub fn subsequent_different() {
        let id1 = ShortId::new();
        let id2 = ShortId::new();

        assert_ne!(id1, id2);
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
        let id: ShortId = Uuid::from_str("001a4929-cd4e-4719-822c-4ec8fe8e743b")
            .unwrap()
            .into();

        assert_eq!(
            "\"00394JKKAE8WCR4B2ES3Z8WX1V\"",
            serde_json::to_string(&id).unwrap()
        );
        assert_eq!(
            id,
            serde_json::from_str("\"00394JKKAE8WCR4B2ES3Z8WX1V\"").unwrap()
        );
    }
}
