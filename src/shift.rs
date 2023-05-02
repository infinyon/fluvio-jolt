use std::borrow::Cow;

use indexmap::IndexMap;
use serde_json::{Map, Value};
use crate::dsl::{LhsWithHash, Lhs, Rhs, RhsEntry, IndexOp};
use crate::spec::Spec;
use crate::{delete, insert, JsonPointer};
use xxhash_rust::xxh3::Xxh3Builder;
use serde::Deserialize;
use crate::transform::Transform;
use crate::{Error, Result};

const ROOT_KEY: &str = "root";

type Obj = IndexMap<LhsWithHash, Val, Xxh3Builder>;

#[derive(Deserialize)]
#[serde(untagged)]
enum Val {
    Obj(Box<Obj>),
    Rhs(Rhs),
}

#[derive(Deserialize)]
pub struct Shift(Obj);

impl Transform for Shift {
    fn apply(&self, val: &Value) -> Result<Value> {
        let mut path = vec![(vec![Cow::Borrowed(ROOT_KEY)], val)];

        let mut out = Value::Null;
        apply(&self.0, &mut path, &mut out)?;

        path.pop().unwrap();
        assert!(path.is_empty());

        Ok(out)
    }
}

fn apply<'ctx, 'input: 'ctx>(
    obj: &'input Obj,
    path: &'ctx mut Vec<(Vec<Cow<'input, str>>, &'input Value)>,
    out: &'ctx mut Value,
) -> Result<()> {
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

fn match_obj_and_key<'ctx, 'input: 'ctx>(
    obj: &'input Obj,
    path: &'ctx mut Vec<(Vec<Cow<'input, str>>, &'input Value)>,
    k: Cow<'input, str>,
    v: &'input Value,
    out: &'ctx mut Value,
) -> Result<()> {
    for (lhs, rhs) in obj.iter() {
        let (res, m) = match_lhs(&lhs.lhs, k.clone(), path)?;
        if let Some(res) = res {
            path.push((m, v));

            let lhs_is_match_all = matches!(res, MatchResult::OutputVal(_));

            match rhs {
                Val::Obj(inner) => {
                    if lhs_is_match_all {
                        return Err(Error::UnexpectedObjectInRhs);
                    }

                    apply(inner, path, out)?;
                }
                Val::Rhs(rhs) => {
                    let v = match res {
                        MatchResult::OutputInputValue => v.clone(),
                        MatchResult::OutputVal(v) => v,
                    };

                    insert_val_to_rhs(rhs, v, out)?;
                }
            }

            path.pop().unwrap();

            if !lhs_is_match_all {
                break;
            }
        }
    }

    Ok(())
}

fn eval_at(at: &Option<(usize, Box<Rhs>)>, path: &[(Vec<Cow<'_, str>>, &Value)]) -> Result<Value> {
    let at = match at {
        Some(at) => at,
        None => return Ok(Value::clone(path.last().unwrap().1)),
    };

    if at.0 >= path.len() {
        return Err(Error::PathIndexOutOfRange {
            idx: at.0,
            len: path.len(),
        });
    }

    let v = &path[path.len() - at.0 - 1];

    eval_rhs(&at.1, &v.1)
}

fn eval_rhs(rhs: &Rhs, v: &Value) -> Result<Value> {
    let mut v = v;

    let mut iter = rhs.0.iter();

    while let Some(entry) = iter.next() {}

    todo!()
}

fn insert_val_to_rhs(rhs: &Rhs, v: Value, out: &mut Value) -> Result<()> {
    todo!()
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
                vec![k],
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
        Lhs::At(at) => {
            let val = eval_at(at, path)?;

            Ok((Some(MatchResult::OutputVal(val)), vec![k]))
        }
        Lhs::Square(lit) => Ok((
            Some(MatchResult::OutputVal(Value::String(lit.to_owned()))),
            vec![k],
        )),
        Lhs::Pipes(pipes) => {
            for stars in pipes.iter() {
                if let Some(m) = match_stars(&stars.0, k.clone()) {
                    return Ok((Some(MatchResult::OutputInputValue), m));
                }
            }
            Ok((None, Vec::new()))
        }
    }
}

fn match_stars<'ctx, 'input: 'ctx>(
    stars: &'input [String],
    k: Cow<'input, str>,
) -> Option<Vec<Cow<'input, str>>> {
    if stars.is_empty() {
        return if k.is_empty() {
            Some(vec!["".into()])
        } else {
            None
        };
    }

    let mut k: Cow<'input, str> = match k {
        Cow::Borrowed(s) => Cow::Borrowed(s.strip_prefix(stars[0].as_str())?),
        Cow::Owned(s) => Cow::Owned(s.strip_prefix(stars[0].as_str())?.to_owned()),
    };

    let mut m = vec![k.clone()];

    for pattern in stars.iter() {
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
        len: path.len(),
    })?;

    Ok(m.clone())
}

pub(crate) fn shift(mut input: Value, spec: &Spec) -> Value {
    let mut result = Value::Object(Map::new());
    for (spec_pointer, spec_leaf) in spec.iter() {
        let target_position = match spec_leaf {
            Value::String(val) => JsonPointer::from_dot_notation(val),
            _ => continue,
        };
        while let Some((input_pointer, input_leaf)) = find_mut(&mut input, &spec_pointer) {
            let mut bindings = input_pointer.entries().to_vec();
            bindings.reverse();
            let mut new_position = target_position.clone();
            new_position.substitute_vars(&bindings);
            insert(&mut result, new_position, input_leaf.take());
            let _ = delete(&mut input, &input_pointer);
        }
    }
    result
}

fn find_mut<'a>(
    dest: &'a mut Value,
    position: &JsonPointer,
) -> Option<(JsonPointer, &'a mut Value)> {
    position.iter().skip(1).try_fold(
        (JsonPointer::default(), dest),
        |(mut path, target), token| match target {
            Value::Object(map) => {
                map.iter_mut()
                    .find(|(k, _)| match_node(k, token))
                    .map(|(k, v)| {
                        path.push(k);
                        (path, v)
                    })
            }
            _ => None,
        },
    )
}

fn match_node(node: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if node == pattern {
        return true;
    }
    if pattern.split('|').any(|part| part == node) {
        return true;
    }
    false
}

#[cfg(test)]
mod test {

    use serde_json::json;
    use super::*;

    #[test]
    fn test_empty_spec() {
        //given
        let spec: Spec = serde_json::from_value(json!({})).expect("parsed spec");

        let input: Value = serde_json::from_value(json!({
            "b" : "b",
            "c" : "c"
        }))
        .expect("parsed spec");

        //when
        let output = shift(input, &spec);

        //then
        assert_eq!(output, json!({}))
    }

    #[test]
    fn test_move_not_present_value() {
        //given
        let spec: Spec = serde_json::from_value(json!({
            "c" : "c"
        }))
        .expect("parsed spec");

        let input: Value = serde_json::from_value(json!({
            "a" : "a",
            "b" : "b"
        }))
        .expect("parsed spec");

        //when
        let output = shift(input, &spec);

        //then
        assert_eq!(output, json!({}))
    }

    #[test]
    fn test_move() {
        //given
        let spec: Spec = serde_json::from_value(json!({
            "c" : "new_c"
        }))
        .expect("parsed spec");

        let input: Value = serde_json::from_value(json!({
            "a" : "a",
            "b" : "b",
            "c" : "c",
        }))
        .expect("parsed spec");

        //when
        let output = shift(input, &spec);

        //then
        assert_eq!(
            output,
            json!({
                "new_c": "c"
            })
        )
    }

    #[test]
    fn test_move_with_wildcard_and_vars() {
        //given
        let spec: Spec = serde_json::from_value(json!({
            "*" : "new.&0"
        }))
        .expect("parsed spec");

        let input: Value = serde_json::from_value(json!({
            "a" : "aa",
            "b" : "bb",
            "c" : "cc",
        }))
        .expect("parsed spec");

        //when
        let output = shift(input, &spec);

        //then
        assert_eq!(
            output,
            json!({
                "new": {
                    "a" : "aa",
                    "b" : "bb",
                    "c" : "cc",
                }
            })
        )
    }

    #[test]
    fn test_move_wildcard_and_static() {
        //given
        let spec: Spec = serde_json::from_value(json!({
            "a" : "new.a",
            "*" : "new.&0"
        }))
        .expect("parsed spec");

        let input: Value = serde_json::from_value(json!({
            "a" : "aa",
            "b" : "bb",
            "c" : "cc",
        }))
        .expect("parsed spec");

        //when
        let output = shift(input, &spec);

        //then
        assert_eq!(
            output,
            json!({
                "new": {
                    "a" : "aa",
                    "b" : "bb",
                    "c" : "cc",
                }
            })
        )
    }

    #[test]
    fn test_wildcard_pointer() {
        //given
        let mut input = json!({
            "a": {
                "b": "b",
                "c": "c"
            }
        });

        //when
        let pointer = find_mut(&mut input, &JsonPointer::from_dot_notation(".a.*"));

        //then
        assert_eq!(
            pointer,
            Some((JsonPointer::from_dot_notation(".a.b"), &mut json!("b")))
        )
    }

    #[test]
    fn test_or_pointer() {
        //given
        let mut input = json!({
            "a": {
                "b": "b",
                "c": "c"
            }
        });

        //when
        let pointer = find_mut(&mut input, &JsonPointer::from_dot_notation(".a.c|d"));

        //then
        assert_eq!(
            pointer,
            Some((JsonPointer::from_dot_notation("a.c"), &mut json!("c")))
        )
    }
}
