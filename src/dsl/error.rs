#[derive(Debug)]
pub struct ParseError {
    pub(crate) pos: usize,
    pub(crate) cause: Box<ParseErrorCause>,
}

#[derive(Debug)]
pub enum ParseErrorCause {
    UnexpectedEndOfInput,
    UnexpectedChar(char),
}
