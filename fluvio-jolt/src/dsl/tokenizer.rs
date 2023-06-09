use super::{
    token::{Token, TokenKind},
    ParseError,
    error::ParseErrorCause,
    chars::Chars,
};

pub struct Tokenizer<'input> {
    chars: Chars<'input>,
    buf: Option<Token>,
}

impl<'input> Tokenizer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            chars: Chars::new(input),
            buf: None,
        }
    }

    pub fn pos(&self) -> usize {
        self.chars.pos()
    }

    fn escape(&mut self) -> Result<char, ParseError> {
        let c = self.chars.next().ok_or(ParseError {
            pos: self.pos(),
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })?;
        if !SPECIAL_CHARS.contains(&c) {
            return Err(ParseError {
                pos: self.pos(),
                cause: Box::new(ParseErrorCause::UnexpectedChar(c)),
            });
        }
        Ok(c)
    }

    fn key(&mut self) -> Result<Token, ParseError> {
        let start = self.pos();
        let mut key = String::new();
        while let Some(c) = self.chars.next() {
            if c == '\\' {
                key.push(self.escape()?);
            } else if SPECIAL_CHARS.contains(&c) {
                self.chars.put_back(c)?;
                break;
            } else {
                key.push(c);
            }
        }

        Ok(Token {
            pos: start,
            kind: TokenKind::Key(key),
        })
    }

    pub fn put_back(&mut self, token: Token) -> Result<(), ParseError> {
        if self.buf.is_some() {
            return Err(ParseError {
                pos: self.pos(),
                cause: Box::new(ParseErrorCause::PutBackBufferFull),
            });
        }

        self.buf = Some(token);

        Ok(())
    }

    pub fn next(&mut self) -> Result<Option<Token>, ParseError> {
        if let Some(token) = self.buf.take() {
            return Ok(Some(token));
        }

        let pos = self.pos();
        let c = match self.chars.next() {
            Some(c) => c,
            None => return Ok(None),
        };

        let token = match c {
            '$' => Token {
                pos,
                kind: TokenKind::DollarSign,
            },
            '&' => Token {
                pos,
                kind: TokenKind::Amp,
            },
            '@' => Token {
                pos,
                kind: TokenKind::At,
            },
            '#' => Token {
                pos,
                kind: TokenKind::Square,
            },
            '*' => Token {
                pos,
                kind: TokenKind::Star,
            },
            '|' => Token {
                pos,
                kind: TokenKind::Pipe,
            },
            '[' => Token {
                pos,
                kind: TokenKind::OpenBrkt,
            },
            ']' => Token {
                pos,
                kind: TokenKind::CloseBrkt,
            },
            '(' => Token {
                pos,
                kind: TokenKind::OpenPrnth,
            },
            ')' => Token {
                pos,
                kind: TokenKind::ClosePrnth,
            },
            '.' => Token {
                pos,
                kind: TokenKind::Dot,
            },
            ',' => Token {
                pos,
                kind: TokenKind::Comma,
            },
            _ => {
                self.chars.put_back(c)?;
                self.key()?
            }
        };

        Ok(Some(token))
    }
}

const SPECIAL_CHARS: [char; 13] = [
    '$', '&', '@', '#', '*', '|', '[', ']', '(', ')', '.', ',', '\\',
];
