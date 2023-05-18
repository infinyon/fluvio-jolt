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
    pub infallible: Vec<(InfallibleLhs, Vec<Rhs>)>,
    pub literal: Vec<(String, REntry)>,
    pub amp: Vec<((usize, usize), REntry)>,
    pub pipes: Vec<(Vec<Stars>, REntry)>,
    pub fn_calls: Vec<((String, Vec<Rhs>), REntry)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum REntry {
    Obj(Box<Object>),
    Rhs(Vec<Rhs>),
    Thrash,
}

struct RhsVisitor;

impl<'de> Visitor<'de> for RhsVisitor {
    type Value = Rhs;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("right hand side expression")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&v)
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
        deserializer.deserialize_any(RhsVisitor)
    }
}

struct RhssVisitor;

impl<'de> Visitor<'de> for RhssVisitor {
    type Value = Vec<Rhs>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Right hand side expression or a array of rhs expressions")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&v)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let r = Rhs::parse(value)
            .map_err(|e| E::custom(format!("failed to parse: {value}.error={e}")))?;
        Ok(vec![r])
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut arr = Vec::new();
        while let Some(rhs) = seq.next_element::<Rhss>()? {
            arr.extend_from_slice(&rhs.0);
        }
        Ok(arr)
    }
}

struct Rhss(Vec<Rhs>);

impl<'de> Deserialize<'de> for Rhss {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(RhssVisitor).map(Rhss)
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

        while let Some(lhs_s) = map.next_key::<String>()? {
            let lhs = LhsVisitor.visit_str(&lhs_s)?;

            if !key_set.insert(lhs_s) {
                return Err(A::Error::custom("duplicate lhs"));
            }

            match lhs {
                Lhs::DollarSign(idx0, idx1) => {
                    obj.infallible.push((
                        InfallibleLhs::DollarSign(idx0, idx1),
                        map.next_value::<Rhss>()?.0,
                    ));
                }
                Lhs::Amp(idx0, idx1) => {
                    obj.amp.push(((idx0, idx1), map.next_value()?));
                }
                Lhs::At(idx, rhs) => {
                    obj.infallible
                        .push((InfallibleLhs::At(idx, rhs), map.next_value::<Rhss>()?.0));
                }
                Lhs::Square(lit) => {
                    obj.infallible
                        .push((InfallibleLhs::Square(lit), map.next_value::<Rhss>()?.0));
                }
                Lhs::Pipes(pipes) => {
                    obj.pipes.push((pipes, map.next_value()?));
                }
                Lhs::Literal(lit) => {
                    obj.literal.push((lit, map.next_value()?));
                }
                Lhs::FnCall(name, args) => {
                    obj.fn_calls.push(((name, args), map.next_value()?));
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
        deserializer.deserialize_map(ObjectVisitor)
    }
}

struct LhsVisitor;

impl<'de> Visitor<'de> for LhsVisitor {
    type Value = Lhs;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Left hand side expression")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&v)
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
        deserializer.deserialize_any(LhsVisitor)
    }
}

struct REntryVisitor;

impl<'de> Visitor<'de> for REntryVisitor {
    type Value = REntry;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Right hand side object or expression")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&v)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        RhsVisitor.visit_str(value).map(|r| REntry::Rhs(vec![r]))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut arr = Vec::new();

        while let Some(rhs) = seq.next_element()? {
            arr.push(rhs);
        }

        Ok(REntry::Rhs(arr))
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        ObjectVisitor
            .visit_map(map)
            .map(|obj| REntry::Obj(Box::new(obj)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(REntry::Thrash)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_none()
    }
}

impl<'de> Deserialize<'de> for REntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(REntryVisitor)
    }
}
