use serde::Deserialize;
use serde_json::Value;
use crate::{JsonPointer, shift::Shift};

/// The JSON transformation specification.
///
/// Composes a list of operation specifications. Each operation has its own DSL (Domain Specific
/// Language) in order to facilitate its narrow job.
///
/// ```
/// use fluvio_jolt::TransformSpec;
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
/// ```
///
/// ### `Shift` operation
/// Specifies where the data from the input JSON should be placed in the output JSON, or in other
/// words, how the input JSON/data should be shifted around to make the output JSON/data.
///
/// At a base level, a single `shift` operation is a mapping from an input path to an output path,
/// similar to the `mv` command in Unix, `mv /var/data /var/backup/data`.
///
/// The input path is a JSON tree structure, and the output path is flattened "dot notation" path
/// notation.
///
///  For example, given this simple input JSON:
///  <pre>
/// {
///     "id": 1,
///     "name": "John Smith",
///     "account": {
///         "id": 1000,
///         "type": "Checking"
///     }
/// }
/// </pre>
/// A simple spec could be constructed by copying that input, and modifying it to supply an output
/// path for each piece of data:
/// <pre>
/// {
///     "id": "data.id",
///     "name": "data.name",
///     "account": "data.account"
/// }
/// </pre>
/// would produce the following output JSON:
/// <pre>
/// {
///     "data" : {
///         "id": 1,
///         "name": "John Smith",
///         "account": {
///             "id": 1000,
///             "type": "Checking"
///         }
///     }
/// }
/// </pre>
/// #### Wildcards
/// The `shift` specification on the keys level supports wildcards and conditions:  
///     1. `*` - match everything  
///     2. `name1|name2|nameN` - match any of the specified names
///
/// `&` lookup allows referencing the values captured by the `*` or `|`.  
/// It allows for specs to be more compact. For example, for this input:
///  <pre>
/// {
///     "id": 1,
///     "name": "John Smith",
///     "account": {
///         "id": 1000,
///         "type": "Checking"
///     }
/// }
/// </pre>
/// to get the output:
/// <pre>
/// {
///     "data" : {
///         "id": 1,
///         "name": "John Smith",
///         "account": {
///             "id": 1000,
///             "type": "Checking"
///         }
///     }
/// }
/// </pre>
/// the spec with wildcards would be:
/// <pre>
/// {
///     "*": "data.&0"
/// }
/// </pre>
/// If you want only `id` and `name` in the output, the spec is:
/// <pre>
/// {
///     "id|name": "data.&0"
/// }
/// </pre>
///
///
/// `&` wildcard also allows to dereference any level of the path of given node:
/// <pre>
/// {
///     "foo": {
///         "bar" : {
///             "baz": "new_location.&0.&1.&2" // &0 = baz, &1 = bar, &2 = foo
///             }
///         }
///     }
/// }
/// </pre>
/// for the input:
/// <pre>
/// {
///     "foo": {
///       "bar": {
///         "baz": "value"
///       }
///     }
///   }
/// </pre>
/// will produce:
/// <pre>
/// {
///     "new_location": {
///       "baz": {
///         "bar": {
///           "foo": "value"
///         }
///       }
///     }
/// }
/// </pre>
///
/// ### `Default` operation
/// Applies default values if the value is not present in the input JSON.
///
///  For example, given this simple input JSON:
///  <pre>
/// {
///     "phones": {
///         "mobile": 01234567,
///         "country": "US"
///     }
/// }
/// </pre>
/// with the following specification for `default` operation:
/// <pre>
/// {
///     "phones": {
///         "mobile": 0000000,
///         "code": "+1"
///     }
/// }
/// </pre>
/// the output JSON will be:
/// <pre>
/// {
///     "phones": {
///         "mobile": 01234567,
///         "country": "US",
///         "code": "+1"
///     }
/// }
/// </pre>
/// As you can see, the field `mobile` remains not affected while the `code` has a default '+1' value.
///
/// ### `Remove` operation
/// Removes content from the input JSON.
/// The spec structure matches the input JSON structure. The value of fields is ignored.
///
///  For example, given this simple input JSON:
///  <pre>
/// {
///     "phones": {
///         "mobile": 01234567,
///         "country": "US"
///     }
/// }
/// </pre>
/// you can remove the `country` by the following specification for `remove` operation:
/// <pre>
/// {
///     "phones": {
///         "country": ""
///     }
/// }
/// </pre>
/// the output JSON will be:
/// <pre>
/// {
///     "phones": {
///         "mobile": 01234567
///     }
/// }
/// </pre>
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct TransformSpec(Vec<SpecEntry>);

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(tag = "operation", content = "spec")]
#[serde(rename_all = "lowercase")]
pub(crate) enum SpecEntry {
    Shift(Shift),
    Default(Spec),
    Remove(Spec),
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub(crate) struct Spec(Value);

#[derive(Debug)]
pub(crate) struct SpecIter<'a> {
    path: Vec<(&'a Value, usize, String)>,
}

impl TransformSpec {
    pub(crate) fn entries(&self) -> impl Iterator<Item = &SpecEntry> {
        self.0.iter()
    }
}

impl Spec {
    pub(crate) fn iter(&self) -> SpecIter {
        SpecIter::new(self)
    }
}

impl<'a> SpecIter<'a> {
    fn new(spec: &'a Spec) -> Self {
        Self {
            path: vec![(&spec.0, 0, String::new())],
        }
    }
}

impl<'a> Iterator for SpecIter<'a> {
    type Item = (JsonPointer, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (current, index, name) = self.path.pop()?;
            match current {
                Value::Array(vec) => {
                    if let Some(next) = vec.get(index) {
                        self.path.push((current, index + 1, name));
                        self.path.push((next, 0, index.to_string()));
                    }
                }
                Value::Object(map) => {
                    if let Some((next_name, next)) = map.iter().nth(index) {
                        self.path.push((current, index + 1, name));
                        self.path.push((next, 0, next_name.clone()));
                    }
                }
                other => {
                    let mut path: Vec<String> =
                        self.path.iter().map(|(_, _, path)| path.clone()).collect();
                    path.push(name);
                    return Some((JsonPointer::new(path), other));
                }
            };
        }
    }
}

#[cfg(test)]
mod test {

    use serde_json::json;
    use super::*;

    #[test]
    fn test_de_from_str() {
        let spec = r#"
        [
            {
                "operation": "shift",
                "spec": {
                    "id": "__data.id",
                    "name": "__data.name",
                    "account": "__data.account"
                }
            }
        ]"#;
        let result: TransformSpec = serde_json::from_str(spec).expect("parsed transform spec");

        assert_eq!(
            result,
            TransformSpec(vec![SpecEntry::Shift(
                serde_json::from_value(json!({
                    "id": "__data.id",
                    "name": "__data.name",
                    "account": "__data.account"
                }))
                .unwrap()
            )])
        );
    }

    // #[test]
    // fn test_spec_iter_preserves_order() {
    //     let spec = r#"
    //     [
    //         {
    //             "operation": "shift",
    //             "spec": {
    //                 "id": "__data.id",
    //                 "name": "__data.name",
    //                 "account": "__data.account",
    //                 "address" : {
    //                     "country": "ext.country",
    //                     "city": "ext.city",
    //                     "phones": ["12345","00000"]
    //                 },
    //                 "*": "&0"
    //             }
    //         }
    //     ]"#;
    //     let result: TransformSpec = serde_json::from_str(spec).expect("parsed transform spec");

    //     let spec_entry = result.entries().next().expect("one spec entry");

    //     let items_vec = spec_entry
    //         .spec
    //         .iter()
    //         .map(|(path, item)| format!("{}:{}", path.join_rfc6901(), item))
    //         .collect::<Vec<String>>();
    //     assert_eq!(
    //         items_vec,
    //         vec![
    //             "/id:\"__data.id\"",
    //             "/name:\"__data.name\"",
    //             "/account:\"__data.account\"",
    //             "/address/country:\"ext.country\"",
    //             "/address/city:\"ext.city\"",
    //             "/address/phones/0:\"12345\"",
    //             "/address/phones/1:\"00000\"",
    //             "/*:\"&0\"",
    //         ]
    //     );
    // }
}
