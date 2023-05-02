use thiserror::Error as ThisError;
use std::result::Result as StdResult;

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
}

pub type Result<T> = StdResult<T, Error>;
