use serde_json::Value as JsonValue;
use crate::Result;

pub trait Transform {
    fn apply(&self, val: &JsonValue) -> Result<JsonValue>;
}
