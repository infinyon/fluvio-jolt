use serde_json::Value;

use serde::{Deserialize, Serialize};
pub struct Transform {}

/// simplest transform
pub fn transform(source: Value, _transform: &Transform) -> Value {
    return source;
}
