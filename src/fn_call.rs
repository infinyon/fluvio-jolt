use std::error::Error;
use serde_json::Value;

pub enum CallableFnResult {
    Matches(Option<Vec<String>>),
    Value(Value),
}

pub type CallableFn = Box<dyn Fn(&[Value]) -> Result<CallableFnResult, Box<dyn Error>>>;
