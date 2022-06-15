use serde::{Serialize, Deserialize};
use serde_json::Value;

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
#[derive(Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
pub struct TransformSpec(Vec<SpecEntry>);

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub(crate) struct SpecEntry {
    pub(crate) operation: Operation,
    pub(crate) spec: Spec,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub(crate) enum Operation {
    Shift,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
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
    type Item = (String, &'a Value);

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
                    let mut path: Vec<&str> =
                        self.path.iter().map(|(_, _, path)| path.as_str()).collect();
                    path.push(name.as_str());
                    return Some((path.join("/"), other));
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
            TransformSpec(vec![SpecEntry {
                operation: Operation::Shift,
                spec: Spec(json!({
                    "id": "__data.id",
                    "name": "__data.name",
                    "account": "__data.account"
                }))
            }])
        );
    }

    #[test]
    fn test_spec_iter() {
        let spec = r#"
        [
            {
                "operation": "shift",
                "spec": {
                    "id": "__data.id",
                    "name": "__data.name",
                    "account": "__data.account",
                    "address" : {
                        "country": "ext.country",
                        "city": "ext.city",
                        "phones": ["12345","00000"]
                    }
                }
            }
        ]"#;
        let result: TransformSpec = serde_json::from_str(spec).expect("parsed transform spec");

        let spec_entry = result.entries().next().expect("one spec entry");

        let items_vec = spec_entry
            .spec
            .iter()
            .map(|(path, item)| format!("{}:{}", path, item))
            .collect::<Vec<String>>();
        assert_eq!(
            items_vec,
            vec![
                "/account:\"__data.account\"",
                "/address/city:\"ext.city\"",
                "/address/country:\"ext.country\"",
                "/address/phones/0:\"12345\"",
                "/address/phones/1:\"00000\"",
                "/id:\"__data.id\"",
                "/name:\"__data.name\"",
            ]
        );
    }
}
