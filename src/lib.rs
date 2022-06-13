mod spec;
mod shift;

use serde_json::Value;

use spec::TransformSpec;
use crate::shift::shift;
use crate::spec::Operation;

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
        let spec = TransformSpec::shift(json!({
            "a": "a_new",
            "c": "c_new"
        }));
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
