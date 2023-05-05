use serde_json::Value as JsonValue;
use crate::Result;

/// Transform interface for individual jolt operations
pub trait Transform {
    /// Apply a transform to an input and get an output value
    fn apply(&self, val: &JsonValue) -> Result<JsonValue>;
}
