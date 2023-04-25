use std::result::Result as StdResult;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Empty expression.")]
    EmptyExpr,
    #[error("Unexpected character in expression: '{0}'")]
    UnexpectedCharacter(char),
    #[error("Unexpected character in expression. Expected '{expected}'. Got '{got}'.")]
    WrongCharacter { expected: char, got: char },
    #[error("Unexpected end of input when parsing expression.")]
    UnexpectedEof,
    #[error("Index too large while parsing an expression. Value was: '{0}'")]
    IndexTooLarge(String),
}

pub type Result<T> = StdResult<T, Error>;
