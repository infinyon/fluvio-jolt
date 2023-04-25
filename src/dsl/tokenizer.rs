use std::str::Chars;
use std::iter::Peekable;
use super::{
    token::{Token, TokenKind},
    ParseError,
    error::ParseErrorCause,
};

pub struct Tokenizer<'input> {
    chars: Peekable<Chars<'input>>,
    byte_idx: usize,
}

impl<'input> Tokenizer<'input> {
    pub fn new(input: &'input str) -> Self {
        let chars = input.chars().peekable();
        Self { chars, byte_idx: 0 }
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.byte_idx += c.len_utf8();
        Some(c)
    }

    fn output_single_char(&mut self, kind: TokenKind) -> Token {
        let pos = self.byte_idx;
        self.advance().unwrap();
        Token { pos, kind }
    }

    fn escape(&mut self) -> Result<char, ParseError> {
        let c = self.advance().ok_or(ParseError {
            pos: self.byte_idx,
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })?;
        if !SPECIAL_CHARS.contains(&c) {
            return Err(ParseError {
                pos: self.byte_idx,
                cause: Box::new(ParseErrorCause::UnexpectedChar(c)),
            });
        }
        Ok(c)
    }

    fn key(&mut self) -> Result<Token, ParseError> {
        let start = self.byte_idx;
        let mut key = String::new();
        loop {
            let c = match self.chars.peek() {
                Some(c) => *c,
                None => break,
            };

            if c == '\\' {
                self.advance().unwrap();
                key.push(self.escape()?);
            } else if SPECIAL_CHARS.contains(&c) {
                break;
            } else {
                key.push(self.advance().unwrap());
            }
        }

        Ok(Token {
            pos: start,
            kind: TokenKind::Key(key),
        })
    }
}

impl<'input> Iterator for Tokenizer<'input> {
    type Item = Result<Token, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.chars.peek()? {
            '$' => Some(Ok(self.output_single_char(TokenKind::DollarSign))),
            '&' => Some(Ok(self.output_single_char(TokenKind::Amp))),
            '@' => Some(Ok(self.output_single_char(TokenKind::At))),
            '#' => Some(Ok(self.output_single_char(TokenKind::Square))),
            '*' => Some(Ok(self.output_single_char(TokenKind::Star))),
            '|' => Some(Ok(self.output_single_char(TokenKind::Pipe))),
            '[' => Some(Ok(self.output_single_char(TokenKind::OpenBrkt))),
            ']' => Some(Ok(self.output_single_char(TokenKind::CloseBrkt))),
            '(' => Some(Ok(self.output_single_char(TokenKind::OpenPrnth))),
            ')' => Some(Ok(self.output_single_char(TokenKind::ClosePrnth))),
            '.' => Some(Ok(self.output_single_char(TokenKind::Dot))),
            ',' => Some(Ok(self.output_single_char(TokenKind::Comma))),
            _ => Some(self.key()),
        }
    }
}

const SPECIAL_CHARS: [char; 13] = [
    '$', '&', '@', '#', '*', '|', '[', ']', '(', ')', '.', ',', '\\',
];
