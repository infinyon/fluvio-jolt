use super::ast::{Rhs, Lhs};
use std::fmt;
use serde::de::{self, Visitor};
use core::hash::Hash;
use serde::{de::Deserializer, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LhsWithHash {
    pub lhs: Lhs,
    pub hash: u64,
}

impl Hash for LhsWithHash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash)
    }
}

struct RhsVisitor;

impl<'de> Visitor<'de> for RhsVisitor {
    type Value = Rhs;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("right hand side expression")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Rhs::parse(value).map_err(|e| E::custom(e.to_string()))
    }
}

struct LhsVisitor;

impl<'de> Visitor<'de> for LhsVisitor {
    type Value = LhsWithHash;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("left hand side expression")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let lhs = Lhs::parse(value).map_err(|e| E::custom(e.to_string()))?;
        let hash = xxhash_rust::xxh3::xxh3_64(value.as_bytes());

        Ok(LhsWithHash { lhs, hash })
    }
}

impl<'de> Deserialize<'de> for LhsWithHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(LhsVisitor)
    }
}

impl<'de> Deserialize<'de> for Rhs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(RhsVisitor)
    }
}
