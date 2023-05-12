use std::str::Chars as StdChars;
use super::error::{ParseErrorCause, ParseError};

pub struct Chars<'input> {
    inner: StdChars<'input>,
    pos: usize,
    buf: Option<char>,
}

impl<'input> Chars<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            inner: input.chars(),
            pos: 0,
            buf: None,
        }
    }

    pub fn put_back(&mut self, c: char) -> Result<(), ParseError> {
        if self.buf.is_some() {
            return Err(ParseError {
                pos: self.pos,
                cause: Box::new(ParseErrorCause::PutBackBufferFull),
            });
        }

        self.buf = Some(c);
        self.pos -= c.len_utf8();

        Ok(())
    }

    pub fn next(&mut self) -> Option<char> {
        let c = match self.buf.take() {
            Some(c) => c,
            None => self.inner.next()?,
        };

        self.pos += c.len_utf8();

        Some(c)
    }

    pub fn pos(&self) -> usize {
        self.pos
    }
}
