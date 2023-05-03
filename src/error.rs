use thiserror::Error as ThisError;
use std::{result::Result as StdResult, num::ParseIntError};

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Path index out of range when using wildcard. Index={idx};Length={len};")]
    PathIndexOutOfRange { idx: usize, len: usize },
    #[error("Match index out of range when using wildcard. Index={idx};Length={len};")]
    MatchIndexOutOfRange { idx: usize, len: usize },
    #[error("Unexpected end of right hand side expression.")]
    UnexpectedEndOfRhs,
    #[error("Unexpected right hand side expression.")]
    UnexpectedRhsEntry,
    #[error("Unexpected object in right hand side.")]
    UnexpectedObjectInRhs,
    #[error("Not implemented yet.")]
    Todo,
    #[error("Invalid index in expression.\n{0}")]
    InvalidIndex(ParseIntError),
    #[error("Array index out of range. Index={idx};Length={len};")]
    ArrIndexOutOfRange { idx: usize, len: usize },
    #[error("Json value can't be used as an index: {0:?}")]
    InvalidIndexVal(serde_json::Value),
    #[error("Key not found in object:{0}")]
    KeyNotFound(String),
    #[error("Expression didn't evaluate to a string.")]
    EvalString,
}

pub type Result<T> = StdResult<T, Error>;
