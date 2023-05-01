use thiserror::Error as ThisError;
use std::result::Result as StdResult;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Path index out of range when using wildcard. Index={idx};Length={len};")]
    PathIndexOutOfRange { idx: usize, len: usize },
    #[error("Match index out of range when using wildcard. Index={idx};Length={len};")]
    MatchIndexOutOfRange { idx: usize, len: usize },
}

pub type Result<T> = StdResult<T, Error>;
