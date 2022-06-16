use serde_json::Value;
use crate::{JsonPointer, delete};
use crate::spec::Spec;

pub(crate) fn remove(mut input: Value, spec: &Spec) -> Value {
    for (path, _) in spec.iter() {
        if input.pointer(&path).is_some() {
            let _ = delete(&mut input, JsonPointer::Rfc6901(&path));
        }
    }
    input
}

#[cfg(test)]
mod test {

    use serde_json::json;
    use super::*;

    #[test]
    fn test_remove_if_absent() {
        //given
        let spec: Spec = serde_json::from_value(json!({
            "a" : "a",
            "d" : {
                "e" : "e"
            }
        }))
        .expect("parsed spec");

        let input: Value = serde_json::from_value(json!({
            "b" : "b",
            "c" : "c"
        }))
        .expect("parsed spec");

        //when
        let output = remove(input, &spec);

        //then
        assert_eq!(
            output,
            json!({
                "b" : "b",
                "c" : "c"
            })
        )
    }

    #[test]
    fn test_remove_if_present() {
        //given
        let spec: Spec = serde_json::from_value(json!({
            "a" : ""
        }))
        .expect("parsed spec");

        let input: Value = serde_json::from_value(json!({
            "a" : "a",
            "b" : "b"
        }))
        .expect("parsed spec");

        //when
        let output = remove(input, &spec);

        //then
        assert_eq!(
            output,
            json!({
                 "b" : "b"
            })
        )
    }
}
