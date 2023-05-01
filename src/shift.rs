use indexmap::IndexMap;
use serde_json::{Map, Value};
use crate::dsl::{LhsWithHash, Lhs, Rhs, RhsEntry};
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
        let mut path = vec![(vec![ROOT_KEY], val)];
        let out = apply(&self.0, &mut path);

        path.pop().unwrap();
        assert!(path.is_empty());

        out
    }
}

fn apply(obj: &Obj, path: &mut Vec<(Vec<&str>, &Value)>) -> Result<Value> {
    let input = path.last().unwrap();

    let input = match input.1 {
        Value::Object(input) => input,
        _ => return Ok(Value::clone(input.1)),
    };

    let mut output = serde_json::Map::new();

    for (k, v) in input.iter() {
        // TODO: apply specific ordering when iterating obj
        for (lhs, rhs) in obj {
            let (res, m) = match_lhs(&lhs.lhs, k, &path)?;
            match res {
                MatchResult::OutputStr(k) => todo!(),
                MatchResult::OutputValue => todo!(),
                MatchResult::OutputRhs => todo!(),
                MatchResult::NoMatch => (),
                MatchResult::OutputAt(idx, rhs_expr) => todo!(),
            }
        }
    }

    Ok(Value::Object(output))
}

enum MatchResult<'a> {
    NoMatch,
    // output this str to the path specified by rhs
    OutputStr(&'a str),
    // output value of input to the path specified by rhs
    OutputValue,
    // evaluate rhs and output the result to input key
    OutputRhs,
    OutputAt(usize, &'a Box<Rhs>),
}

fn match_lhs<'a>(
    lhs: &'a Lhs,
    k: &'a str,
    path: &'a Vec<(Vec<&'a str>, &'a Value)>,
) -> Result<(MatchResult<'a>, Vec<&'a str>)> {
    match lhs {
        Lhs::DollarSign(path_idx, match_idx) => {
            let m = get_match((*path_idx, *match_idx), path)?;
            Ok((MatchResult::OutputStr(m), vec![k]))
        }
        Lhs::Amp(path_idx, match_idx) => {
            let m = get_match((*path_idx, *match_idx), path)?;
            if m == k {
                Ok((MatchResult::OutputRhs, vec![k]))
            } else {
                Ok((MatchResult::NoMatch, Vec::new()))
            }
        }
        Lhs::At(at) => match at {
            Some((idx, rhs)) => Ok((MatchResult::OutputAt(*idx, rhs), vec![k])),
            None => Ok((MatchResult::OutputValue, vec![k])),
        },
        Lhs::Square(lit) => Ok((MatchResult::OutputStr(lit), vec![k])),
        Lhs::Pipes(pipes) => {
            for stars in pipes.iter() {
                if let Some(m) = match_stars(&stars.0, k) {
                    return Ok((MatchResult::OutputRhs, m));
                }
            }
            Ok((MatchResult::NoMatch, Vec::new()))
        }
    }
}

fn match_stars<'a>(stars: &'a [String], k: &'a str) -> Option<Vec<&'a str>> {
    if stars.is_empty() {
        return None;
    }

    let mut k = k.strip_prefix(stars[0].as_str())?;

    let mut m = Vec::new();

    for pattern in stars.iter() {
        if pattern.is_empty() {
            m.push(k);
        } else {
            match k.find(pattern.as_str()) {
                None => return None,
                Some(idx) => {
                    m.push(&k[..idx]);
                    k = &k[idx..];
                }
            }
        }
    }

    Some(m)
}

fn get_match<'a>(idx: (usize, usize), path: &'a Vec<(Vec<&'a str>, &'a Value)>) -> Result<&'a str> {
    let (matches, _) = path.get(idx.0).ok_or(Error::PathIndexOutOfRange {
        idx: idx.0,
        len: path.len(),
    })?;
    let m = matches.get(idx.1).ok_or(Error::MatchIndexOutOfRange {
        idx: idx.1,
        len: path.len(),
    })?;

    Ok(*m)
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
