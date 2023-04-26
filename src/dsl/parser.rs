use super::error::{ParseErrorCause, ParseError};
use super::token::TokenKind;
use super::tokenizer::Tokenizer;
use std::result::Result as StdResult;
use super::ast::{Lhs, Rhs, IndexOp, RhsEntry, KeySelection};

type Result<T> = StdResult<T, ParseError>;

pub struct Parser<'input> {
    input: Tokenizer<'input>,
}

impl<'input> Parser<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            input: Tokenizer::new(input),
        }
    }

    pub fn parse_lhs(&mut self) -> Result<Lhs> {
        let token = self.input.peek().ok_or(ParseError {
            pos: self.input.pos(),
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })??;

        let res = match token.kind {
            TokenKind::Square => self.parse_square_lhs().map(Lhs::Square),
            TokenKind::At => self.parse_at().map(Lhs::At),
            TokenKind::DollarSign => self.parse_dollar_sign().map(|t| Lhs::DollarSign(t.0, t.1)),
            TokenKind::Amp => self.parse_amp().map(|t| Lhs::Amp(t.0, t.1)),
            _ => self.parse_key_selection().map(Lhs::Key),
        }?;

        if let Some(token) = self.input.next() {
            let token = token?;
            return Err(ParseError {
                pos: token.pos,
                cause: Box::new(ParseErrorCause::UnexpectedToken(token)),
            });
        }

        Ok(res)
    }

    pub fn parse_rhs(&mut self) -> Result<Rhs> {
        let mut entries = Vec::new();

        while let Some(token) = self.input.peek() {
            let token = token?;
            let res = match token.kind {
                TokenKind::Amp => self.parse_amp().map(|t| RhsEntry::Amp(t.0, t.1)),
                TokenKind::At => self.parse_at().map(RhsEntry::At),
                TokenKind::OpenBrkt => self.parse_index_op().map(RhsEntry::Index),
                TokenKind::Dot => {
                    self.assert_next(TokenKind::Dot)?;
                    Ok(RhsEntry::Dot)
                }
                TokenKind::Key(key) => Ok(RhsEntry::Key(key)),
                _ => {
                    return Err(ParseError {
                        pos: token.pos,
                        cause: Box::new(ParseErrorCause::UnexpectedToken(
                            self.input.next().unwrap().unwrap(),
                        )),
                    });
                }
            }?;

            entries.push(res);
        }

        Ok(Rhs(entries))
    }

    fn assert_next(&mut self, expected: TokenKind) -> Result<()> {
        let got = self.input.next().ok_or(ParseError {
            pos: self.input.pos(),
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })??;
        if expected == got.kind {
            Ok(())
        } else {
            return Err(ParseError {
                pos: got.pos,
                cause: Box::new(ParseErrorCause::UnexpectedToken(got)),
            });
        }
    }

    fn parse_index_op(&mut self) -> Result<IndexOp> {
        self.assert_next(TokenKind::OpenBrkt)?;

        let token = self.input.peek().ok_or(ParseError {
            pos: self.input.pos(),
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })??;

        let op = match token.kind {
            TokenKind::Square => {
                self.assert_next(TokenKind::Square)?;
                let idx = self.parse_index()?;
                IndexOp::Square(idx)
            }
            TokenKind::Amp => {
                let amp = self.parse_amp()?;
                IndexOp::Amp(amp.0, amp.1)
            }
            TokenKind::CloseBrkt => IndexOp::Empty,
            TokenKind::Key(key) => {
                self.input.next().unwrap().unwrap();

                let idx = key.parse().map_err(|e| ParseError {
                    pos: token.pos,
                    cause: Box::new(ParseErrorCause::InvalidIndex(e)),
                })?;

                IndexOp::Literal(idx)
            }
            _ => {
                return Err(ParseError {
                    pos: token.pos,
                    cause: Box::new(ParseErrorCause::UnexpectedToken(
                        self.input.next().unwrap().unwrap(),
                    )),
                });
            }
        };

        self.assert_next(TokenKind::CloseBrkt)?;

        Ok(op)
    }

    fn parse_square_lhs(&mut self) -> Result<String> {
        self.assert_next(TokenKind::Square)?;

        let token = self.input.next().ok_or(ParseError {
            pos: self.input.pos(),
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })??;

        match token.kind {
            TokenKind::Key(key) => Ok(key),
            _ => Err(ParseError {
                pos: token.pos,
                cause: Box::new(ParseErrorCause::UnexpectedToken(
                    self.input.next().unwrap().unwrap(),
                )),
            }),
        }
    }

    fn parse_at(&mut self) -> Result<Option<(usize, String)>> {
        self.assert_next(TokenKind::At)?;

        let token = match self.input.peek() {
            Some(token) => token?,
            None => return Ok(None),
        };

        match token.kind {
            TokenKind::OpenPrnth => (),
            _ => return Ok(None),
        }

        self.assert_next(TokenKind::OpenPrnth)?;
        let idx = self.parse_index()?;
        self.assert_next(TokenKind::Comma)?;

        let token = self.input.next().ok_or(ParseError {
            pos: self.input.pos(),
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })??;

        let key = match token.kind {
            TokenKind::Key(key) => key,
            _ => {
                return Err(ParseError {
                    pos: token.pos,
                    cause: Box::new(ParseErrorCause::UnexpectedToken(token)),
                });
            }
        };

        self.assert_next(TokenKind::ClosePrnth)?;

        Ok(Some((idx, key)))
    }

    fn parse_dollar_sign(&mut self) -> Result<(usize, usize)> {
        self.assert_next(TokenKind::DollarSign)?;
        self.parse_amp_or_ds()
    }

    fn parse_amp(&mut self) -> Result<(usize, usize)> {
        self.assert_next(TokenKind::Amp)?;
        self.parse_amp_or_ds()
    }

    fn parse_amp_or_ds(&mut self) -> Result<(usize, usize)> {
        let c = match self.chars.peek() {
            Some(c) => *c,
            None => return Ok((0, 0)),
        };

        if c.is_ascii_digit() {
            let idx = self.parse_index()?;
            Ok((idx, 0))
        } else if c == '(' {
            self.assert_next('(')?;
            let idx0 = self.parse_index()?;
            self.assert_next(',')?;
            let idx1 = self.parse_index()?;
            self.assert_next(')')?;

            Ok((idx0, idx1))
        } else {
            Err(Error::UnexpectedCharacter(c))
        }
    }

    fn parse_key_selection(&mut self) -> Result<KeySelection> {
        enum State {
            Literal(String),
            Pipe(String, Vec<String>),
            Stars(String, Vec<String>),
        }

        let mut state = State::Literal(String::new());

        for c in self.chars.by_ref() {
            match c {
                '*' => match state {
                    State::Literal(buf) => {
                        state = State::Stars(String::new(), vec![buf]);
                    }
                    State::Pipe(mut buf, bufs) => {
                        return Err(ParseError {});
                    }
                    State::Stars(buf, mut bufs) => {
                        bufs.push(buf);
                        state = State::Stars(String::new(), bufs);
                    }
                },
                '|' => match state {
                    State::Literal(buf) => {
                        state = State::Pipe(String::new(), vec![buf]);
                    }
                    State::Pipe(buf, mut bufs) => {
                        bufs.push(buf);
                        state = State::Pipe(String::new(), bufs);
                    }
                    State::Stars(mut buf, bufs) => {
                        return Err(ParseError {});
                    }
                },
                _ => match state {
                    State::Literal(mut buf) => {
                        buf.push(c);
                        state = State::Literal(buf);
                    }
                    State::Pipe(mut buf, bufs) => {
                        buf.push(c);
                        state = State::Pipe(buf, bufs);
                    }
                    State::Stars(mut buf, bufs) => {
                        buf.push(c);
                        state = State::Stars(buf, bufs);
                    }
                },
            }
        }

        Ok(match state {
            State::Literal(buf) => KeySelection::Literal(buf),
            State::Pipe(buf, mut bufs) => {
                bufs.push(buf);
                KeySelection::Pipe(bufs)
            }
            State::Stars(buf, mut bufs) => {
                bufs.push(buf);

                if bufs.len() == 2 && bufs.iter().all(String::is_empty) {
                    KeySelection::Star
                } else {
                    KeySelection::Stars(bufs)
                }
            }
        })
    }

    fn parse_index(&mut self) -> Result<usize> {
        let mut num = String::new();
        while let Some(c) = self.chars.peek() {
            let c = *c;
            if !c.is_ascii_digit() {
                break;
            }
            self.assert_next(c)?;

            num.push(c);
        }

        if num.is_empty() {
            return Err(Error::EmptyNumber);
        }

        num.parse().map_err(|_| Error::IndexTooLarge(num))
    }
}
