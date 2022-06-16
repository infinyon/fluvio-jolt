use serde_json::Value;
use crate::{insert, JsonPointer};
use crate::spec::Spec;

pub(crate) fn default(mut input: Value, spec: &Spec) -> Value {
    for (path, leaf) in spec.iter() {
        if input.pointer(&path).is_none() {
            let _ = insert(&mut input, JsonPointer::Rfc6901(&path), leaf.clone());
        }
    }
    input
}

#[cfg(test)]
mod test {

    use serde_json::json;
    use super::*;

    #[test]
    fn test_insert_if_absent() {
        //given
        let spec: Spec = serde_json::from_value(json!({
            "a" : "default_value",
            "d" : {
                "e" : "default_value"
            }
        }))
        .expect("parsed spec");

        let input: Value = serde_json::from_value(json!({
            "b" : "b",
            "c" : "c"
        }))
        .expect("parsed spec");

        //when
        let output = default(input, &spec);

        //then
        assert_eq!(
            output,
            json!({
                "a" : "default_value",
                "b" : "b",
                "c" : "c",
                "d" : {
                    "e" : "default_value"
                }
            })
        )
    }

    #[test]
    fn test_skip_insert_if_present() {
        //given
        let spec: Spec = serde_json::from_value(json!({
            "a" : "default_value"
        }))
        .expect("parsed spec");

        let input: Value = serde_json::from_value(json!({
            "a" : "a",
            "b" : "b"
        }))
        .expect("parsed spec");

        //when
        let output = default(input, &spec);

        //then
        assert_eq!(
            output,
            json!({
                "a" : "a",
                "b" : "b"
            })
        )
    }
}
