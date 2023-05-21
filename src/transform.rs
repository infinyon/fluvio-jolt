use std::collections::HashMap;

use serde_json::Value as JsonValue;
use crate::{
    Result,
    fn_call::{CallableFn, CallableFnResult},
    Error,
};

/// Transform interface for individual jolt operations
pub trait Transform {
    /// Apply a transform to an input and get an output value
    fn apply(&self, ctx: &Context, val: &JsonValue) -> Result<JsonValue>;
}

#[derive(Default)]
pub struct Context {
    callable_fns: HashMap<String, CallableFn>,
}

impl Context {
    pub fn register_fn<S: Into<String>>(&mut self, key: S, callable_fn: CallableFn) -> Result<()> {
        let key = key.into();
        if self.callable_fns.contains_key(&key) {
            return Err(Error::FunctionAlreadyRegistered(key));
        }

        self.callable_fns.insert(key, callable_fn);

        Ok(())
    }

    pub fn deregister_fn<S: AsRef<str>>(&mut self, key: S) -> Option<CallableFn> {
        self.callable_fns.remove(key.as_ref())
    }

    fn call_fn(&self, key: &str, args: &[JsonValue]) -> Result<CallableFnResult> {
        let f = self
            .callable_fns
            .get(key)
            .ok_or_else(|| Error::FunctionNotFound(key.to_owned()))?;

        f(args).map_err(Error::FunctionError)
    }

    pub(crate) fn call_fn_get_matches<S: AsRef<str>>(
        &self,
        key: S,
        args: &[JsonValue],
    ) -> Result<Option<Vec<String>>> {
        match self.call_fn(key.as_ref(), args)? {
            CallableFnResult::Matches(m) => Ok(m),
            _ => Err(Error::FunctionResultUnexpected(key.as_ref().to_owned())),
        }
    }

    pub(crate) fn call_fn_get_value<S: AsRef<str>>(
        &self,
        key: S,
        args: &[JsonValue],
    ) -> Result<JsonValue> {
        match self.call_fn(key.as_ref(), args)? {
            CallableFnResult::Value(v) => Ok(v),
            _ => Err(Error::FunctionResultUnexpected(key.as_ref().to_owned())),
        }
    }
}
