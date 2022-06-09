use serde_json::Value;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Transform {

}


/// simplest transform
pub fn transform(source: Value, _transform: &Transform) -> Value {

    return source;

}