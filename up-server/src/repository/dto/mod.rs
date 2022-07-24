use std::{fmt::Debug, hash::Hash, str::FromStr};

pub mod account;
pub mod check;

/// Represents a field in the data dto (can be used in queries, parse from
/// strings, converted to strings, and used as map keys).
pub trait ModelField: Debug + Clone + Hash + PartialEq + Eq + FromStr + AsRef<str> {}
