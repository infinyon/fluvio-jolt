use std::borrow::Cow;

use indexmap::IndexMap;
use serde_json::Value;
use xxhash_rust::xxh3::Xxh3Builder;
use serde::Deserialize;

use crate::dsl::{LhsWithHash, Lhs, Rhs, RhsEntry, IndexOp, RhsPart};
use crate::transform::Transform;
use crate::{Error, Result};

const ROOT_KEY: &str = "root";

type Obj = IndexMap<LhsWithHash, Val, Xxh3Builder>;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
enum Val {
    Obj(Box<Obj>),
    Rhs(Rhs),
    Arr(Vec<Rhs>),
    Null,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Shift(Obj);

impl Transform for Shift {
    fn apply(&self, val: &Value) -> Result<Value> {
        let mut path = vec![(vec![Cow::Borrowed(ROOT_KEY)], val)];

        let mut out = Value::Null;
        apply(&self.0, &mut path, &mut out)?;

        path.pop().unwrap();
        // path should always be empty at this point
        // if not, the implementation is broken
        assert!(path.is_empty());

        Ok(out)
    }
}

// Apply an object from spec to the input
// input is passed using the path and the current input should be
// at the tip of the path
fn apply<'ctx, 'input: 'ctx>(
    obj: &'input Obj,
    path: &'ctx mut Vec<(Vec<Cow<'input, str>>, &'input Value)>,
    out: &'ctx mut Value,
) -> Result<()> {
    // run the infallible lhs exprs first
    match_obj_and_key_impl(
        obj,
        path,
        path.last().unwrap().0[0].clone(),
        path.last().unwrap().1,
        out,
        LhsSelection::Infallible,
    )?;
    let input = path.last().unwrap();

    match input.1 {
        Value::Object(input) => {
            for (k, v) in input.iter() {
                match_obj_and_key(obj, path, Cow::Borrowed(k), v, out)?;
            }
        }
        Value::Bool(b) => {
            let k = if *b { "true" } else { "false" };

            match_obj_and_key(obj, path, Cow::Borrowed(k), input.1, out)?;
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

            match_obj_and_key(obj, path, Cow::Owned(k), input.1, out)?;
        }
        Value::String(k) => {
            match_obj_and_key(obj, path, Cow::Borrowed(k), input.1, out)?;
        }
        Value::Null => {
            let k = "null";
            match_obj_and_key(obj, path, Cow::Borrowed(k), input.1, out)?;
        }
    };

    Ok(())
}

// Match and object in the spec with a key/value pair from the input
// This function only runs the k/v pairs that have a fallible lhs in the spec
// The infallible ones should be run sparately
fn match_obj_and_key<'ctx, 'input: 'ctx>(
    obj: &'input Obj,
    path: &'ctx mut Vec<(Vec<Cow<'input, str>>, &'input Value)>,
    k: Cow<'input, str>,
    v: &'input Value,
    out: &'ctx mut Value,
) -> Result<()> {
    if match_obj_and_key_impl(obj, path, k.clone(), v, out, LhsSelection::Literal)? {
        // Return early if we already matched a k/v pair
        return Ok(());
    }
    if match_obj_and_key_impl(obj, path, k.clone(), v, out, LhsSelection::Amp)? {
        return Ok(());
    }
    if match_obj_and_key_impl(obj, path, k.clone(), v, out, LhsSelection::Pipes)? {
        return Ok(());
    }

    Ok(())
}

fn lhs_is_fallible(lhs: &Lhs) -> bool {
    !matches!(lhs, Lhs::DollarSign(_, _) | Lhs::Square(_) | Lhs::At(_, _))
}

#[derive(PartialEq)]
enum LhsSelection {
    // Infallible lhs exprs include
    // @, # and $
    Infallible,
    // Literal lhs like `my_key`
    Literal,
    // Amp lhs `&`
    Amp,
    // Lhs including Pipes and stars like `hello*|*world`
    Pipes,
}

// match an object in the spec with a k/v pair in the input
// has a filter to dictate which kind of lhs expr are ran
// this allows to run the lhs exprs in a specific order
// We should ideally do this before the execution time but it is
// implemented like this to make it simpler for now
fn match_obj_and_key_impl<'ctx, 'input: 'ctx>(
    obj: &'input Obj,
    path: &'ctx mut Vec<(Vec<Cow<'input, str>>, &'input Value)>,
    k: Cow<'input, str>,
    v: &'input Value,
    out: &'ctx mut Value,
    // Used to filter the lhs expressions that are used for matching
    // It is necessary because there is a sepcific order of Lhs expression
    // types to process according to the original implementation of jolt
    selection: LhsSelection,
) -> Result<bool> {
    let mut matched = false;

    for (lhs, rhs) in obj.iter() {
        match selection {
            LhsSelection::Infallible => {
                if lhs_is_fallible(&lhs.lhs) {
                    continue;
                }
            }
            LhsSelection::Literal => {
                if !matches!(lhs.lhs, Lhs::Literal(_)) {
                    continue;
                }
            }
            LhsSelection::Amp => {
                if !matches!(lhs.lhs, Lhs::Amp(_, _)) {
                    continue;
                }
            }
            LhsSelection::Pipes => {
                if !matches!(lhs.lhs, Lhs::Pipes(_)) {
                    continue;
                }
            }
        }
        let (res, m) = match_lhs(&lhs.lhs, k.clone(), path)?;
        if let Some(res) = res {
            matched = true;

            path.push((m, v));

            match rhs {
                Val::Obj(inner) => {
                    if selection == LhsSelection::Infallible {
                        return Err(Error::UnexpectedObjectInRhs);
                    }

                    apply(inner, path, out)?;
                }
                Val::Rhs(rhs) => {
                    let v = match res {
                        MatchResult::OutputInputValue => v.clone(),
                        MatchResult::OutputVal(v) => v,
                    };

                    insert_val_to_rhs(rhs, v, path, out)?;
                }
                Val::Arr(rhs_arr) => {
                    let v = match res {
                        MatchResult::OutputInputValue => v.clone(),
                        MatchResult::OutputVal(v) => v,
                    };

                    for rhs in rhs_arr.iter() {
                        insert_val_to_rhs(rhs, v.clone(), path, out)?;
                    }
                }
                Val::Null => (),
            }

            path.pop().unwrap();

            if selection != LhsSelection::Infallible {
                break;
            }
        }
    }

    Ok(matched)
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
    let obj = v.as_object().ok_or_else(|| {
        dbg!(&v);
        Error::UnexpectedRhsEntry
    })?;

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

                dbg!(entries);
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

#[derive(PartialEq)]
enum MatchResult {
    // output value of input to the path specified by rhs if rhs is an expression
    // if rhs is an object keep going down the tree
    OutputInputValue,
    // output this value to the path specified by the right hand side
    // the right hand side must be an expression
    OutputVal(Value),
}

fn match_lhs<'ctx, 'input: 'ctx>(
    lhs: &'input Lhs,
    k: Cow<'input, str>,
    path: &'ctx [(Vec<Cow<'input, str>>, &'input Value)],
) -> Result<(Option<MatchResult>, Vec<Cow<'input, str>>)> {
    match lhs {
        Lhs::DollarSign(path_idx, match_idx) => {
            let m = get_match((*path_idx, *match_idx), path)?;
            Ok((
                Some(MatchResult::OutputVal(Value::String(m.into()))),
                get_matches(0, path)?.to_vec(),
            ))
        }
        Lhs::Amp(path_idx, match_idx) => {
            let m = get_match((*path_idx, *match_idx), path)?;
            if m == k {
                Ok((Some(MatchResult::OutputInputValue), vec![k]))
            } else {
                Ok((None, Vec::new()))
            }
        }
        Lhs::At(idx, rhs) => {
            let val = eval_at((*idx, rhs), path)?;

            Ok((
                Some(MatchResult::OutputVal(val)),
                get_matches(0, path)?.to_vec(),
            ))
        }
        Lhs::Square(lit) => Ok((
            Some(MatchResult::OutputVal(Value::String(lit.to_owned()))),
            get_matches(0, path)?.to_vec(),
        )),
        Lhs::Pipes(pipes) => {
            for stars in pipes.iter() {
                if let Some(m) = match_stars(&stars.0, k.clone()) {
                    return Ok((Some(MatchResult::OutputInputValue), m));
                }
            }
            Ok((None, Vec::new()))
        }
        Lhs::Literal(lit) => {
            if lit == k.as_ref() {
                Ok((Some(MatchResult::OutputInputValue), vec![k]))
            } else {
                Ok((None, Vec::new()))
            }
        }
    }
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

fn get_matches<'ctx, 'input: 'ctx>(
    idx: usize,
    path: &'ctx [(Vec<Cow<'input, str>>, &'input Value)],
) -> Result<&'ctx [Cow<'input, str>]> {
    if idx >= path.len() {
        return Err(Error::PathIndexOutOfRange {
            idx,
            len: path.len(),
        });
    }

    Ok(&path[path.len() - idx - 1].0)
}
