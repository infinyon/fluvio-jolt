mod spec;
mod shift;

use serde_json::Value;

use crate::shift::shift;
use crate::spec::Operation;

pub use spec::TransformSpec;

/// Perform JSON to JSON transformation
pub fn transform(input: Value, spec: &TransformSpec) -> Value {
    let mut result = input;
    for entry in spec.entries() {
        match entry.operation {
            Operation::Shift => result = shift(result, &entry.spec),
        }
    }
    result
}

#[cfg(test)]
mod test {

    use serde_json::json;
    use super::*;

    #[test]
    fn test_transform() {
        let spec: TransformSpec = serde_json::from_value(json!(
            [
                {
                  "operation": "shift",
                  "spec": {
                    "a": "a_new",
                    "c": "c_new"
                  }
                }
            ]
        ))
        .expect("parsed spec");

        let source = json!({
            "a": "b",
            "c": "d"
        });
        let result = transform(source, &spec);

        assert_eq!(
            result,
            json!({
                "a_new": "b",
                "c_new": "d"
            })
        );
    }
}
