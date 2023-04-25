use std::fmt;
use std::error::Error;
use thiserror::Error as ThisError;

#[derive(Debug)]
pub struct ParseError {
    pub(crate) pos: usize,
    pub(crate) cause: Box<ParseErrorCause>,
}

#[derive(Debug, ThisError)]
pub enum ParseErrorCause {
    #[error("Unexpected end of input.")]
    UnexpectedEndOfInput,
    #[error("Unexpected character: '{0}'.")]
    UnexpectedChar(char),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to parse input. Error near {}.\n{}",
            self.pos, self.cause
        )
    }
}

impl Error for ParseError {}
