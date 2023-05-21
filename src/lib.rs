mod spec;
mod shift;
mod default;
mod remove;
mod pointer;
mod transform;
mod error;
mod fn_call;
#[cfg(not(feature = "fuzz"))]
mod dsl;
#[cfg(feature = "fuzz")]
pub mod dsl;

use serde_json::{Map, Value};
use serde_json::map::Entry;
use transform::Transform;

use crate::default::default;
use crate::remove::remove;
use crate::spec::SpecEntry;

pub use spec::TransformSpec;
use crate::pointer::JsonPointer;

pub use error::{Error, Result};

pub use fn_call::{CallableFn, CallableFnResult, Matcher, Processor};
pub use transform::Context;

/// Perform JSON to JSON transformation where the "specification" is a JSON.
///
/// Inspired by Java library [Jolt](https://github.com/bazaarvoice/jolt).
///
/// The transformation can compose of many operations that are chained together.
///
/// ### Operations
/// 1. [`shift`](TransformSpec#shift-operation): copy data from the input tree and put it the output tree
/// 2. [`default`](TransformSpec#default-operation): apply default values to the tree
/// 3. [`remove`](TransformSpec#remove-operation): remove data from the tree
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
/// let output = transform(input, &spec).unwrap();
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
pub fn transform(input: Value, spec: &TransformSpec) -> Result<Value> {
    let mut result = input;
    for entry in spec.entries() {
        match entry {
            SpecEntry::Shift(shift) => result = shift.apply(&Default::default(), &result)?,
            SpecEntry::Default(spec) => result = default(result, spec),
            SpecEntry::Remove(spec) => result = remove(result, spec),
        }
    }
    Ok(result)
}

pub(crate) fn insert(dest: &mut Value, position: JsonPointer, val: Value) {
    let elements = position.iter();
    let folded = elements
        .skip(1)
        .try_fold(dest, |target, token| match target {
            Value::Object(map) => {
                if let Entry::Vacant(entry) = map.entry(token) {
                    entry.insert(Value::Object(Map::new()));
                }
                map.get_mut(token)
            }
            _ => None,
        });
    if let Some(pointer_mut) = folded {
        merge(pointer_mut, val);
    }
}

/// Merge one `Value` node into another if they are both `Value::Object`, otherwise overwrite.
fn merge(dest: &mut Value, new_value: Value) {
    match (dest, new_value) {
        (Value::Object(dest), Value::Object(new_value)) => {
            for (key, value) in new_value.into_iter() {
                dest.insert(key, value);
            }
        }
        (dest, new_value) => *dest = new_value,
    };
}

pub(crate) fn delete(dest: &mut Value, position: &JsonPointer) -> Option<()> {
    if let Some(Value::Object(map)) = dest.pointer_mut(position.parent().join_rfc6901().as_str()) {
        map.remove(position.leaf_name());
    }
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
        let result = transform(source, &spec).unwrap();

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
        let mut empty_dest = Value::Object(Map::new());
        let value = json!({
            "a": "b",
        });

        insert(
            &mut empty_dest,
            JsonPointer::from_dot_notation("new"),
            value,
        );

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
        let mut dest = json!({
            "b": "bb",
            "c": "cc",
        });
        let value = json!({
            "a": "b",
        });

        insert(&mut dest, JsonPointer::from_dot_notation("new"), value);

        assert_eq!(
            dest,
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
    fn test_insert_object_merged() {
        //given
        let mut dest = json!({
            "some": {
                "b": "bb",
                "c": "cc",
            }
        });
        let value = json!({
            "a": "b",
        });

        insert(&mut dest, JsonPointer::from_dot_notation("some"), value);

        assert_eq!(
            dest,
            json!({
                "some": {
                    "a": "b",
                    "b": "bb",
                    "c": "cc",
                }
            })
        );
    }

    #[test]
    fn test_insert_object_to_empty_non_root() {
        //given
        let mut empty_dest = Value::Object(Map::new());
        let value = json!({
            "a": "b",
        });

        insert(
            &mut empty_dest,
            JsonPointer::from_dot_notation("level1.level2.new"),
            value,
        );

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

    #[test]
    fn test_delete_empty_pointer() {
        //given
        let mut input = json!({
            "a": "b",
        });

        //when
        let _ = delete(&mut input, &JsonPointer::from_dot_notation(""));

        //then
        assert_eq!(
            input,
            json!({
                "a": "b",
            })
        );
    }

    #[test]
    fn test_delete_not_existing() {
        //given
        let mut input = json!({
            "a": "b",
        });

        //when
        let _ = delete(&mut input, &JsonPointer::from_dot_notation(".b"));

        //then
        assert_eq!(
            input,
            json!({
                "a": "b",
            })
        );
    }

    #[test]
    fn test_delete() {
        //given
        let mut input1 = json!({
            "a": "b",
        });
        let mut input2 = json!({
            "a": "b",
            "b": "c",
        });
        //when
        let _ = delete(&mut input1, &JsonPointer::from_dot_notation(".a"));
        let _ = delete(&mut input2, &JsonPointer::from_dot_notation("b"));

        //then
        assert_eq!(input1, json!({}));
        assert_eq!(
            input2,
            json!({
                "a": "b",
            })
        );
    }
}
