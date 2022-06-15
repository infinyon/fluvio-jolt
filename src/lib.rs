mod spec;
mod shift;

use serde_json::Value;

use crate::shift::shift;
use crate::spec::Operation;

pub use spec::TransformSpec;

/// Perform JSON to JSON transformation where the "specification" is a JSON.
///
/// Inspired by Java library [Jolt](https://github.com/bazaarvoice/jolt).
///
/// The transformation can compose of many operations that are chained together.
///
/// ### Operations
/// 1. [`shift`](TransformSpec#shift-operation): copy data from the input tree and put it the output tree
/// 2. `default`: apply default values to the tree (not implemented yet)
/// 3. `remove`: remove data from the tree (not implemented yet)
///
/// For example, if you want to repack your JSON record, you can do the following:
/// ```
/// use serde_json::{json, Value};
/// use fluvio_jolt::{transform, TransformSpec};
///
/// let input: Value = serde_json::from_str(r#"
///     {
///         "id": 1,
///         "name": "John Smith",
///         "account": {
///             "id": 1000,
///             "type": "Checking"
///         }
///     }
/// "#).unwrap();
///
/// let spec: TransformSpec =
/// serde_json::from_str(r#"[
///     {
///       "operation": "shift",
///       "spec": {
///         "name": "data.name",
///         "account": "data.account"
///       }
///     }
///   ]"#).unwrap();
///
/// let output = transform(input, &spec);
///
/// assert_eq!(output, json!({
///     "data" : {
///       "name": "John Smith",
///       "account": {
///         "id": 1000,
///         "type": "Checking"
///       }
///     }
/// }));
/// ```
///
/// Checkout supported operations in [TransformSpec] docs.
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
