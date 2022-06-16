use serde_json::{Map, Value};
use crate::spec::Spec;
use crate::{insert, JsonPointer};

pub(crate) fn shift(mut input: Value, spec: &Spec) -> Value {
    let mut result = Value::Object(Map::new());
    for (path, leaf) in spec.iter() {
        let new_position = match leaf {
            Value::String(val) => val,
            _ => continue,
        };
        if let Some(input_leaf) = input.pointer_mut(&path) {
            let _ = insert(
                &mut result,
                JsonPointer::DotNotation(new_position),
                input_leaf.take(),
            );
        }
    }
    result
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
}
