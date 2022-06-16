mod spec;
mod shift;
mod default;

use serde_json::{Map, Value};
use serde_json::map::Entry;

use crate::shift::shift;
use crate::default::default;
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
/// 2. [`default`](TransformSpec#default-operation): apply default values to the tree
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
            Operation::Default => result = default(result, &entry.spec),
        }
    }
    result
}

pub(crate) enum JsonPointer<'a> {
    DotNotation(&'a str),
    Rfc6901(&'a str),
}

impl<'a> JsonPointer<'a> {
    /// Returns path elements of the pointer. First element is always empty string that corresponds
    /// to root level.
    fn elements(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = match self {
            JsonPointer::DotNotation(str) => str.split('.').collect(),
            JsonPointer::Rfc6901(str) => str.split('/').collect(),
        };
        if paths.get(0).filter(|p| (**p).eq("")).is_none() {
            paths.insert(0, "");
        }
        paths
    }
}

pub(crate) fn insert(dest: &mut Value, position: JsonPointer, val: Value) -> Option<()> {
    let paths = position.elements();
    for i in 0..paths.len() - 1 {
        let ancestor = dest.pointer_mut(paths[0..=i].join("/").as_str())?;
        let child_name = paths[i + 1];
        let child_object = if i < paths.len() - 2 {
            Value::Object(Map::new())
        } else {
            Value::Null
        };
        match ancestor {
            Value::Object(ref mut map) => {
                if let Entry::Vacant(entry) = map.entry(child_name) {
                    entry.insert(child_object);
                }
            }
            other => {
                let mut map = Map::new();
                map.insert(child_name.to_string(), child_object);
                *other = Value::Object(map);
            }
        };
    }
    let pointer_mut = dest.pointer_mut(paths.join("/").as_str())?;
    *pointer_mut = val;
    Some(())
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

    #[test]
    fn test_insert_object_to_empty() {
        //given
        let mut empty_dest = Value::Null;
        let value = json!({
            "a": "b",
        });

        let result = insert(&mut empty_dest, JsonPointer::DotNotation("new"), value);

        assert!(result.is_some());
        assert_eq!(
            empty_dest,
            json!({
                "new": {
                    "a": "b"
                }
            })
        );
    }

    #[test]
    fn test_insert_object_to_non_empty() {
        //given
        let mut empty_dest = json!({
            "b": "bb",
            "c": "cc",
        });
        let value = json!({
            "a": "b",
        });

        let result = insert(&mut empty_dest, JsonPointer::DotNotation("new"), value);

        assert!(result.is_some());
        assert_eq!(
            empty_dest,
            json!({
                "b": "bb",
                "c": "cc",
                "new": {
                    "a": "b"
                }
            })
        );
    }

    #[test]
    fn test_insert_object_to_empty_non_root() {
        //given
        let mut empty_dest = Value::Null;
        let value = json!({
            "a": "b",
        });

        let result = insert(
            &mut empty_dest,
            JsonPointer::DotNotation("level1.level2.new"),
            value,
        );

        assert!(result.is_some());
        assert_eq!(
            empty_dest,
            json!({
                "level1": {
                    "level2": {
                        "new": {
                            "a": "b"
                        }
                    }
                }
            })
        );
    }
}
