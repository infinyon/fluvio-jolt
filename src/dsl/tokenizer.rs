use std::str::Chars;
use std::iter::Peekable;
use super::{
    token::{Token, TokenKind},
    ParseError,
    error::ParseErrorCause,
};

pub struct Tokenizer<'input> {
    chars: Peekable<Chars<'input>>,
    pos: usize,
    cache: Option<Token>,
}

impl<'input> Tokenizer<'input> {
    pub fn new(input: &'input str) -> Self {
        let chars = input.chars().peekable();
        Self {
            chars,
            pos: 0,
            cache: None,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.pos += 1;
        Some(c)
    }

    fn output_single_char(&mut self, kind: TokenKind) -> Token {
        let pos = self.pos;
        self.advance().unwrap();
        Token { pos, kind }
    }

    fn escape(&mut self) -> Result<char, ParseError> {
        let c = self.advance().ok_or(ParseError {
            pos: self.pos,
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })?;
        if !SPECIAL_CHARS.contains(&c) {
            return Err(ParseError {
                pos: self.pos,
                cause: Box::new(ParseErrorCause::UnexpectedChar(c)),
            });
        }
        Ok(c)
    }

    fn key(&mut self) -> Result<Token, ParseError> {
        let start = self.pos;
        let mut key = String::new();
        while let Some(&c) = self.chars.peek() {
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

    pub fn peek(&mut self) -> Option<Result<&Token, ParseError>> {
        if self.cache.is_none() {
            self.cache = match self.next() {
                Some(Ok(v)) => Some(v),
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            };
        }

        Some(Ok(self.cache.as_ref().unwrap()))
    }

    // check if it is possible to get an index value from the tip of input
    // intended to be used with `get_idx`
    pub fn can_get_idx(&mut self) -> Option<Result<bool, ParseError>> {
        self.peek().map(|res| {
            res.map(|token| match &token.kind {
                TokenKind::Key(k) => k
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false),
                _ => false,
            })
        })
    }

    // Parses a usize value from the tip of the input and returns it
    // This is a hacky function introduced to implement parsing something like
    // `&123abc` which is interpreted as &123 + abc by the original implementation
    // Maybe the parsing can be fixed to do this more naturally in the future.
    // This function panics if called without checking with `can_get_idx` first.
    pub fn get_idx(&mut self) -> usize {
        let token = self.next().unwrap().unwrap();

        match token.kind {
            TokenKind::Key(k) => {
                let mut idx = 0;

                for (i, c) in k.char_indices() {
                    if !c.is_ascii_digit() {
                        break;
                    } else {
                        idx = i;
                    }
                }

                let mut k = k;

                let rest = k.split_off(idx + 1);

                if !rest.is_empty() {
                    self.cache = Some(Token {
                        pos: token.pos,
                        kind: TokenKind::Key(rest),
                    });
                }

                k[..idx + 1].parse().unwrap()
            }
            _ => panic!("can't get idx from tokenizer"),
        }
    }
}

impl<'input> Iterator for Tokenizer<'input> {
    type Item = Result<Token, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.cache.take() {
            return Some(Ok(token));
        }

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
