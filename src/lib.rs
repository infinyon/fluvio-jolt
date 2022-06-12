use serde_json::Value;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Transform {}

/// Perform JSON to JSOn transformation
pub fn jolt(source: &Value, _transform: &Transform) -> Value {
    source.clone()
}

#[cfg(test)]
mod test {

    use serde_json::json;
    use super::*;

    #[test]
    fn test_transform() {
        let transform = Transform {};
        let source = json!({
            "a": "b",
            "c": "d"
        });
        let result = jolt(&source, &transform);

        assert_eq!(result, source);
    }
}
