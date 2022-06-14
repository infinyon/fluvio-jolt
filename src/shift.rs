use serde_json::{Map, Value};
use serde_json::map::Entry;
use crate::spec::Spec;

pub(crate) fn shift(mut input: Value, spec: &Spec) -> Value {
    let mut result = Value::Object(Map::new());
    for (path, leaf) in spec.iter() {
        let new_position = match leaf {
            Value::String(val) => val,
            _ => continue,
        };
        if let Some(input_leaf) = input.pointer_mut(&path) {
            let _ = insert(&mut result, new_position, input_leaf.take());
        }
    }
    result
}

fn insert(dest: &mut Value, position: &str, val: Value) -> Option<()> {
    let mut paths = position.split('.').collect::<Vec<&str>>();
    paths.insert(0, "");
    for i in 0..paths.len() - 1 {
        let ancestor = dest.pointer_mut(paths[0..=i].join("/").as_str())?;
        let next_name = paths[i + 1];
        let next_object = if i < paths.len() - 2 {
            Value::Object(Map::new())
        } else {
            Value::Null
        };
        match ancestor {
            Value::Object(ref mut map) => {
                if let Entry::Vacant(entry) = map.entry(next_name) {
                    entry.insert(next_object);
                }
            }
            other => {
                let mut map = Map::new();
                map.insert(next_name.to_string(), next_object);
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
    fn test_insert_object_to_empty() {
        //given
        let mut empty_dest = Value::Null;
        let value = json!({
            "a": "b",
        });

        let result = insert(&mut empty_dest, "new", value);

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

        let result = insert(&mut empty_dest, "new", value);

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

        let result = insert(&mut empty_dest, "level1.level2.new", value);

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
