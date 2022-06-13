use serde::{Serialize, Deserialize};
use serde_json::Value;

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

impl TransformSpec {
    pub(crate) fn shift(spec: Value) -> Self {
        Self(vec![SpecEntry {
            operation: Operation::Shift,
            spec: Spec(spec),
        }])
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = &SpecEntry> {
        self.0.iter()
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
}
