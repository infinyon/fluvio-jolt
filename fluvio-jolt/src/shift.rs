use std::borrow::Cow;

use serde_json::Value;
use serde::Deserialize;

use crate::dsl::{Object, REntry, InfallibleLhs, Rhs, RhsEntry, IndexOp, RhsPart};
use crate::transform::Transform;
use crate::{Error, Result};

const ROOT_KEY: &str = "root";

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Shift(Object);

impl Transform for Shift {
    fn apply(&self, val: &Value) -> Result<Value> {
        let mut path = vec![(vec![Cow::Borrowed(ROOT_KEY)], val)];

        let mut out = Value::Null;
        apply(&self.0, &mut path, &mut out)?;

        path.pop().ok_or(Error::ShiftEmptyPath)?;
        // path should always be empty at this point
        // if not, the implementation is broken
        if !path.is_empty() {
            return Err(Error::ShiftPathNotEmpty);
        }

        Ok(out)
    }
}

// Apply an object from spec to the input
// input is passed using the path and the current input should be
// at the tip of the path
fn apply<'ctx, 'input: 'ctx>(
    obj: &'input Object,
    path: &'ctx mut Vec<(Vec<Cow<'input, str>>, &'input Value)>,
    out: &'ctx mut Value,
) -> Result<()> {
    let tip = path.last().ok_or(Error::ShiftEmptyPath)?.clone();

    for (lhs, rhs) in obj.infallible.iter() {
        let v = match lhs {
            InfallibleLhs::DollarSign(idx0, idx1) => {
                let s = get_match((*idx0, *idx1), path)?;
                Value::String(s.into())
            }
            InfallibleLhs::At(idx, rhs) => eval_at((*idx, rhs), path)?,
            InfallibleLhs::Square(lit) => Value::String(lit.clone()),
        };

        path.push(tip.clone());
        for rhs in rhs.iter() {
            insert_val_to_rhs(rhs, v.clone(), path, out)?;
        }
        path.pop().ok_or(Error::ShiftEmptyPath)?;
    }

    match tip.1 {
        Value::Object(input) => {
            for (k, v) in input.iter() {
                match_obj_and_key(obj, path, Cow::Borrowed(k), v, out)?;
            }
        }
        Value::Bool(b) => {
            let k = if *b { "true" } else { "false" };

            match_obj_and_key(obj, path, Cow::Borrowed(k), tip.1, out)?;
        }
        Value::Array(arr) => {
            for (k, v) in arr.iter().enumerate() {
                let k = k.to_string();
                match_obj_and_key(
                    obj,
                    path,
                    // this makes the downstream functions to do some extra allocations.
                    // could avoid some of these allocations by mapping some small indexes to static str's
                    Cow::Owned(k),
                    v,
                    out,
                )?;
            }
        }
        Value::Number(n) => {
            let k = n.to_string();

            match_obj_and_key(obj, path, Cow::Owned(k), tip.1, out)?;
        }
        Value::String(k) => {
            match_obj_and_key(obj, path, Cow::Borrowed(k), tip.1, out)?;
        }
        Value::Null => {
            let k = "null";
            match_obj_and_key(obj, path, Cow::Borrowed(k), tip.1, out)?;
        }
    };

    Ok(())
}

// Match and object in the spec with a key/value pair from the input
// This function only runs the k/v pairs that have a fallible lhs in the spec
// The infallible ones should have ran beforehand
fn match_obj_and_key<'ctx, 'input: 'ctx>(
    obj: &'input Object,
    path: &'ctx mut Vec<(Vec<Cow<'input, str>>, &'input Value)>,
    k: Cow<'input, str>,
    v: &'input Value,
    out: &'ctx mut Value,
) -> Result<()> {
    for (lit, rhs) in obj.literal.iter() {
        let lit = Cow::Borrowed(lit.as_ref());
        if lit == k {
            path.push((vec![lit], v));
            apply_match(v, rhs, path, out)?;
            path.pop().ok_or(Error::ShiftEmptyPath)?;
            return Ok(());
        }
    }

    for (amp, rhs) in obj.amp.iter() {
        let m = get_match(*amp, path)?;
        if m == k {
            path.push((vec![m], v));
            apply_match(v, rhs, path, out)?;
            path.pop().ok_or(Error::ShiftEmptyPath)?;
            return Ok(());
        }
    }

    for (pipes, rhs) in obj.pipes.iter() {
        for stars in pipes.iter() {
            if let Some(m) = match_stars(&stars.0, Cow::clone(&k)) {
                path.push((m, v));
                apply_match(v, rhs, path, out)?;
                path.pop().ok_or(Error::ShiftEmptyPath)?;
                return Ok(());
            }
        }
    }

    Ok(())
}

fn apply_match<'ctx, 'input: 'ctx>(
    v: &'input Value,
    rhs: &'input REntry,
    path: &'ctx mut Vec<(Vec<Cow<'input, str>>, &'input Value)>,
    out: &'ctx mut Value,
) -> Result<()> {
    match rhs {
        REntry::Obj(object) => apply(object, path, out),
        REntry::Rhs(rhs) => {
            for rhs in rhs.iter() {
                insert_val_to_rhs(rhs, v.clone(), path, out)?;
            }
            Ok(())
        }
        REntry::Thrash => Ok(()),
    }
}

// Evaluate an @ expression into a json value using the given path
fn eval_at(at: (usize, &Rhs), path: &[(Vec<Cow<'_, str>>, &Value)]) -> Result<Value> {
    if at.0 >= path.len() {
        return Err(Error::PathIndexOutOfRange {
            idx: at.0,
            len: path.len(),
        });
    }

    let v = &path[path.len() - at.0 - 1];

    eval_rhs(at.1, v.1, path)
}

// Evaluate a rhs expression into a json value using the given path
fn eval_rhs(rhs: &Rhs, v: &Value, path: &[(Vec<Cow<'_, str>>, &Value)]) -> Result<Value> {
    let mut v = v;

    for part in rhs.0.iter() {
        match part {
            RhsPart::Index(idx_op) => match v {
                Value::Array(a) => {
                    let idx = match idx_op {
                        IndexOp::Amp(idx0, idx1) => {
                            let m = get_match((*idx0, *idx1), path)?;
                            m.parse().map_err(Error::InvalidIndex)?
                        }
                        IndexOp::Literal(idx) => *idx,
                        IndexOp::At(idx, rhs) => match eval_at((*idx, rhs), path)? {
                            Value::Number(n) => n
                                .clone()
                                .as_u64()
                                .ok_or(Error::InvalidIndexVal(Value::Number(n.clone())))?
                                .try_into()
                                .map_err(|_| Error::InvalidIndexVal(Value::Number(n)))?,
                            Value::String(s) => s.parse().map_err(Error::InvalidIndex)?,
                            v => return Err(Error::InvalidIndexVal(v)),
                        },
                        IndexOp::Empty => {
                            return Err(Error::UnexpectedRhsEntry);
                        }
                    };

                    v = a
                        .get(idx)
                        .ok_or(Error::ArrIndexOutOfRange { idx, len: a.len() })?;
                }
                _ => {
                    return Err(Error::UnexpectedRhsEntry);
                }
            },
            RhsPart::CompositeKey(entries) => {
                let mut key = String::new();

                for entry in entries {
                    let cow = rhs_entry_to_cow(entry, path)?;
                    key += cow.as_ref();
                }

                v = key_into_object(v, &key)?;
            }
            RhsPart::Key(entry) => {
                let cow = rhs_entry_to_cow(entry, path)?;
                v = key_into_object(v, cow.as_ref())?;
            }
        }
    }

    Ok(Value::clone(v))
}

// Evaluate a rhs expression into a string
fn rhs_entry_to_cow<'ctx, 'input: 'ctx>(
    entry: &'input RhsEntry,
    path: &'ctx [(Vec<Cow<'input, str>>, &'input Value)],
) -> Result<Cow<'input, str>> {
    let cow = match entry {
        RhsEntry::Amp(idx0, idx1) => get_match((*idx0, *idx1), path)?,
        RhsEntry::At(idx, rhs) => {
            let key = eval_at((*idx, rhs), path)?;
            match key {
                Value::String(s) => Cow::Owned(s),
                Value::Number(n) => Cow::Owned(n.to_string()),
                Value::Bool(b) => {
                    if b {
                        Cow::Borrowed("true")
                    } else {
                        Cow::Borrowed("false")
                    }
                }
                _ => return Err(Error::EvalString),
            }
        }
        RhsEntry::Key(key) => Cow::Borrowed(key.as_str()),
    };

    Ok(cow)
}

// index into an object using a given key
// errors if key is not found
fn key_into_object<'input>(v: &'input Value, key: &str) -> Result<&'input Value> {
    let obj = v.as_object().ok_or(Error::UnexpectedRhsEntry)?;

    match obj.get(key) {
        Some(v) => Ok(v),
        None => Err(Error::KeyNotFound(key.to_owned())),
    }
}

fn insert_val_to_rhs<'ctx, 'input: 'ctx>(
    rhs: &Rhs,
    v: Value,
    path: &'ctx [(Vec<Cow<'input, str>>, &'input Value)],
    out: &mut Value,
) -> Result<()> {
    let mut out = out;

    for part in rhs.0.iter() {
        match part {
            RhsPart::Index(idx_op) => {
                let arr = if out.is_array() {
                    out.as_array_mut().unwrap()
                } else if out.is_null() {
                    *out = Value::Array(Vec::new());
                    out.as_array_mut().unwrap()
                } else {
                    *out = Value::Array(vec![std::mem::take(out)]);
                    out.as_array_mut().unwrap()
                };

                let idx = match idx_op {
                    IndexOp::Amp(idx0, idx1) => {
                        let m = get_match((*idx0, *idx1), path)?;
                        m.parse().map_err(Error::InvalidIndex)?
                    }
                    IndexOp::Literal(idx) => *idx,
                    IndexOp::At(idx, rhs) => match eval_at((*idx, rhs), path)? {
                        Value::Number(n) => n
                            .clone()
                            .as_u64()
                            .ok_or(Error::InvalidIndexVal(Value::Number(n.clone())))?
                            .try_into()
                            .map_err(|_| Error::InvalidIndexVal(Value::Number(n)))?,
                        Value::String(s) => s.parse().map_err(Error::InvalidIndex)?,
                        v => return Err(Error::InvalidIndexVal(v)),
                    },
                    IndexOp::Empty => {
                        arr.push(Value::Null);
                        out = arr.last_mut().unwrap();
                        continue;
                    }
                };

                while arr.len() <= idx {
                    arr.push(Value::Null);
                }

                out = arr.get_mut(idx).unwrap();
            }
            RhsPart::CompositeKey(entries) => {
                let mut key = String::new();

                for entry in entries {
                    let cow = rhs_entry_to_cow(entry, path)?;
                    key += cow.as_ref();
                }

                let obj = if out.is_object() {
                    out.as_object_mut().unwrap()
                } else {
                    *out = Value::Object(Default::default());
                    out.as_object_mut().unwrap()
                };

                out = obj.entry(&key).or_insert(Value::Null);
            }
            RhsPart::Key(entry) => {
                let cow = rhs_entry_to_cow(entry, path)?;
                let obj = if out.is_object() {
                    out.as_object_mut().unwrap()
                } else {
                    *out = Value::Object(Default::default());
                    out.as_object_mut().unwrap()
                };

                out = obj.entry(cow.as_ref()).or_insert(Value::Null);
            }
        }
    }

    match out {
        Value::Null => {
            *out = v;
        }
        Value::Array(arr) => {
            arr.push(v);
        }
        val => {
            let v = Value::Array(vec![std::mem::take(val), v]);
            *val = v;
        }
    }

    Ok(())
}

fn match_stars<'ctx, 'input: 'ctx>(
    stars: &'input [String],
    k: Cow<'input, str>,
) -> Option<Vec<Cow<'input, str>>> {
    match stars.len() {
        0 => {
            return if k.is_empty() {
                Some(vec!["".into()])
            } else {
                None
            };
        }
        1 => {
            return if k == stars[0].as_str() {
                Some(vec![k])
            } else {
                None
            };
        }
        _ => (),
    }

    let mut m = vec![k.clone()];

    let prefix = stars[0].as_str();

    let mut k = if prefix.is_empty() {
        k
    } else {
        match k {
            Cow::Borrowed(s) => {
                let s = s.strip_prefix(prefix)?;
                Cow::Borrowed(s)
            }
            Cow::Owned(s) => {
                let s = s.strip_prefix(prefix)?;
                Cow::Owned(s.to_owned())
            }
        }
    };

    for pattern in stars.iter().skip(1) {
        if !pattern.is_empty() {
            match k.find(pattern.as_str()) {
                None => return None,
                Some(idx) => match &k {
                    Cow::Borrowed(s) => {
                        m.push(Cow::Borrowed(&s[..idx]));
                        k = Cow::Borrowed(&s[idx..]);
                    }
                    Cow::Owned(s) => {
                        m.push(Cow::Owned(s[..idx].to_owned()));
                        k = Cow::Owned(s[idx..].to_owned());
                    }
                },
            }
        } else {
            m.push(k.clone());
        }
    }

    Some(m)
}

fn get_match<'ctx, 'input: 'ctx>(
    idx: (usize, usize),
    path: &'ctx [(Vec<Cow<'input, str>>, &'input Value)],
) -> Result<Cow<'input, str>> {
    if idx.0 >= path.len() {
        return Err(Error::PathIndexOutOfRange {
            idx: idx.0,
            len: path.len(),
        });
    }

    let (matches, _) = &path[path.len() - idx.0 - 1];

    let m = matches.get(idx.1).ok_or(Error::MatchIndexOutOfRange {
        idx: idx.1,
        len: matches.len(),
    })?;

    Ok(m.clone())
}
