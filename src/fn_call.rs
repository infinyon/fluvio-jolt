use std::error::Error;
use serde_json::Value;

pub enum CallableFnResult {
    Matcher(Matcher),
    Processor(Processor),
}

pub type CallableFn = Box<dyn Fn(&[Value]) -> Result<CallableFnResult, Box<dyn Error>>>;
pub type Processor = Box<dyn Fn(Value) -> Result<Value, Box<dyn Error>>>;
pub type Matcher = Box<dyn Fn(&str) -> Result<Option<Vec<&str>>, Box<dyn Error>>>;
