use std::collections::HashSet;
use std::fmt;

use serde::de::{self, Visitor};
use serde::{
    de::{Error as _, Deserializer},
    Deserialize,
};

use super::ast::{Rhs, Lhs, Stars};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InfallibleLhs {
    DollarSign(usize, usize),
    At(usize, Box<Rhs>),
    Square(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Object {
    infallible: Vec<(InfallibleLhs, Rhs)>,
    literal: Vec<(String, REntry)>,
    amp: Vec<((usize, usize), REntry)>,
    pipes: Vec<(Vec<Stars>, REntry)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum REntry {
    Obj(Box<Object>),
    Rhs(Rhs),
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
        Rhs::parse(value).map_err(|e| E::custom(format!("failed to parse: {value}.error={e}")))
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

struct ObjectVisitor;

impl<'de> Visitor<'de> for ObjectVisitor {
    type Value = Object;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
        A::Error: de::Error,
    {
        let mut obj = Object::default();

        let mut key_set = HashSet::new();

        while let Some(lhs) = map.next_key()? {
            if !key_set.insert(lhs) {
                return Err(A::Error::custom("duplicate lhs"));
            }

            let lhs = LhsVisitor.visit_str(lhs)?;

            match lhs {
                Lhs::DollarSign(idx0, idx1) => {
                    obj.infallible
                        .push((InfallibleLhs::DollarSign(idx0, idx1), map.next_value()?));
                }
                Lhs::Amp(idx0, idx1) => {
                    obj.amp.push(((idx0, idx1), map.next_value()?));
                }
                Lhs::At(idx, rhs) => {
                    obj.infallible
                        .push((InfallibleLhs::At(idx, rhs), map.next_value()?));
                }
                Lhs::Square(lit) => {
                    obj.infallible
                        .push((InfallibleLhs::Square(lit), map.next_value()?));
                }
                Lhs::Pipes(pipes) => {
                    obj.pipes.push((pipes, map.next_value()?));
                }
                Lhs::Literal(lit) => {
                    obj.literal.push((lit, map.next_value()?));
                }
            }
        }

        Ok(obj)
    }
}

impl<'de> Deserialize<'de> for Object {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ObjectVisitor)
    }
}

struct LhsVisitor;

impl<'de> Visitor<'de> for LhsVisitor {
    type Value = Lhs;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Lhs expression")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Lhs::parse(value).map_err(|e| E::custom(format!("failed to parse: {value}.error={e}")))
    }
}

impl<'de> Deserialize<'de> for Lhs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(LhsVisitor)
    }
}

struct REntryVisitor;

impl<'de> Visitor<'de> for REntryVisitor {
    type Value = REntry;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Right hand side entry")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        RhsVisitor.visit_str(value).map(REntry::Rhs)
    }
}

impl<'de> Deserialize<'de> for REntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(REntryVisitor)
    }
}
