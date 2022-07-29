use serde_json::{Map, Value};
use crate::spec::Spec;
use crate::{delete, insert, JsonPointer};

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
