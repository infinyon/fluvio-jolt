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
        let mut path = vec![(vec![ROOT_KEY], val)];

        let mut out = serde_json::Map::new();
        apply(&self.0, &mut path, &mut out)?;

        path.pop().unwrap();
        assert!(path.is_empty());

        Ok(Value::Object(out))
    }
}

fn apply<'b, 'a: 'b>(
    obj: &'a Obj,
    path: &'b mut Vec<(Vec<&'a str>, &'a Value)>,
    out: &'b mut serde_json::Map<String, Value>,
) -> Result<()> {
    let input = path.last().unwrap();

    match input.1 {
        Value::Object(input) => {
            for (k, v) in input.iter() {
                match_obj_and_key(obj, path, k, Some(v), out)?;
            }
        }
        Value::Bool(b) => {
            let k = if *b { "true" } else { "false" };

            match_obj_and_key(obj, path, k, None, out)?;
        }
        Value::Array(arr) => {
            for (k, v) in arr.iter().enumerate() {
                let k = k.to_string();
                match_obj_and_key(obj, path, &k, Some(v), out)?;
            }
        }
        Value::Number(n) => {
            let k = n.to_string();

            match_obj_and_key(obj, path, &k, None, out)?;
        }
        Value::String(k) => {
            match_obj_and_key(obj, path, k, None, out)?;
        }
        Value::Null => {
            let k = "null";
            match_obj_and_key(obj, path, k, None, out)?;
        }
    };

    Ok(())
}

fn match_obj_and_key<'b, 'a: 'b>(
    obj: &'a Obj,
    path: &'b mut Vec<(Vec<&'a str>, &'a Value)>,
    k: &'a str,
    v: Option<&'a Value>,
    out: &'b mut serde_json::Map<String, Value>,
) -> Result<Value> {
    todo!()
}

fn eval_rhs_target<'a>(val: &Val, path: &mut [(Vec<&str>, &'a Value)]) -> Result<&'a mut Value> {
    match val {
        Val::Rhs(rhs) => eval_rhs_to_ref(rhs, path),
        Val::Obj(obj) => Err(Error::UnexpectedObjectInRhs),
    }
}

fn eval_rhs_expr(rhs: &Rhs, path: &mut [(Vec<&str>, &Value)]) -> Result<Value> {
    eval_rhs_to_ref(rhs, path).map(|v| Value::clone(v))
}

fn eval_rhs_to_ref<'a>(rhs: &Rhs, path: &mut [(Vec<&str>, &'a Value)]) -> Result<&'a mut Value> {
    let mut iter = rhs.0.iter();

    while let Some(entry) = iter.next() {
        match entry {
            RhsEntry::Dot => (),
            RhsEntry::Index(index_op) => match index_op {
                IndexOp::Square(idx) => {}
                IndexOp::Amp(idx0, idx1) => {}
                IndexOp::Literal(idx) => {}
                IndexOp::Empty => {}
            },
            _ => return Err(Error::UnexpectedRhsEntry),
        }

        let entry = iter.next().ok_or(Error::UnexpectedEndOfRhs)?;

        match entry {
            RhsEntry::Amp(idx0, idx1) => {
                let m = get_match((*idx0, *idx1), path)?;
            }
            RhsEntry::At(at) => {}
            RhsEntry::Key(key) => {}
            _ => return Err(Error::UnexpectedRhsEntry),
        }
    }

    Ok(current)
}

#[derive(PartialEq)]
enum MatchResult {
    NoMatch,
    // output value of input to the path specified by rhs if rhs is an expression
    // if rhs is an object keep going down the tree
    OutputInputValue,
    // output this value to the path specified by the right hand side
    // the right hand side must be an expression
    OutputVal(Value),
}

fn match_lhs<'b, 'a: 'b>(
    lhs: &'a Lhs,
    k: &'a str,
    path: &'b [(Vec<&'a str>, &'a Value)],
) -> Result<(MatchResult, Vec<&'a str>)> {
    match lhs {
        Lhs::DollarSign(path_idx, match_idx) => {
            let m = get_match((*path_idx, *match_idx), path)?;
            Ok((MatchResult::OutputVal(Value::String(m.to_owned())), vec![k]))
        }
        Lhs::Amp(path_idx, match_idx) => {
            let m = get_match((*path_idx, *match_idx), path)?;
            if m == k {
                Ok((MatchResult::OutputInputValue, vec![k]))
            } else {
                Ok((MatchResult::NoMatch, Vec::new()))
            }
        }
        Lhs::At(at) => {
            let val = match at {
                Some(at) => todo!(),
                None => Value::clone(path.last().unwrap().1),
            };

            Ok((MatchResult::OutputVal(val), vec![k]))
        }
        Lhs::Square(lit) => Ok((
            MatchResult::OutputVal(Value::String(lit.to_owned())),
            vec![k],
        )),
        Lhs::Pipes(pipes) => {
            for stars in pipes.iter() {
                if let Some(m) = match_stars(&stars.0, k) {
                    return Ok((MatchResult::OutputInputValue, m));
                }
            }
            Ok((MatchResult::NoMatch, Vec::new()))
        }
    }
}

fn match_stars<'a>(stars: &'a [String], k: &'a str) -> Option<Vec<&'a str>> {
    if stars.is_empty() {
        return if k.is_empty() { Some(vec![""]) } else { None };
    }

    let mut k = k.strip_prefix(stars[0].as_str())?;

    let mut m = vec![k];

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

fn get_match<'b, 'a: 'b>(
    idx: (usize, usize),
    path: &'b [(Vec<&'a str>, &'a Value)],
) -> Result<&'a str> {
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
