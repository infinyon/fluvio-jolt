use super::error::{ParseErrorCause, ParseError};
use super::token::TokenKind;
use super::tokenizer::Tokenizer;
use std::result::Result as StdResult;
use super::ast::{Lhs, Rhs, IndexOp, RhsEntry, Stars};

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
        let pos = self.input.pos();
        let token = self.input.peek().ok_or(ParseError {
            pos,
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })??;

        let res = match token.kind {
            TokenKind::Square => self.parse_square_lhs().map(Lhs::Square),
            TokenKind::At => self.parse_at().map(Lhs::At),
            TokenKind::DollarSign => self.parse_dollar_sign().map(|t| Lhs::DollarSign(t.0, t.1)),
            TokenKind::Amp => self.parse_amp().map(|t| Lhs::Amp(t.0, t.1)),
            _ => self.parse_pipes().map(Lhs::Pipes),
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
            let res = match &token.kind {
                TokenKind::Amp => self.parse_amp().map(|t| RhsEntry::Amp(t.0, t.1)),
                TokenKind::At => self.parse_at().map(RhsEntry::At),
                TokenKind::OpenBrkt => self.parse_index_op().map(RhsEntry::Index),
                TokenKind::Dot => {
                    self.assert_next(TokenKind::Dot)?;
                    Ok(RhsEntry::Dot)
                }
                TokenKind::Key(_) => self.parse_key().map(RhsEntry::Key),
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
            Err(ParseError {
                pos: got.pos,
                cause: Box::new(ParseErrorCause::UnexpectedToken(got)),
            })
        }
    }

    fn parse_key(&mut self) -> Result<String> {
        let token = self.input.next().ok_or(ParseError {
            pos: self.input.pos(),
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })??;

        match token.kind {
            TokenKind::Key(key) => Ok(key),
            _ => Err(ParseError {
                pos: self.input.pos(),
                cause: Box::new(ParseErrorCause::UnexpectedToken(token)),
            }),
        }
    }

    fn parse_index_op(&mut self) -> Result<IndexOp> {
        self.assert_next(TokenKind::OpenBrkt)?;

        let pos = self.input.pos();
        let token = self.input.peek().ok_or(ParseError {
            pos,
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })??;

        let op = match &token.kind {
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
            TokenKind::Key(_) => {
                let idx = self.parse_index()?;
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
        let token = match self.input.peek() {
            Some(token) => token,
            None => return Ok((0, 0)),
        }?;

        match &token.kind {
            TokenKind::Key(key) => {
                if key.chars().all(|c| c.is_ascii_digit()) {
                    let idx = self.parse_index()?;
                    Ok((idx, 0))
                } else {
                    Ok((0, 0))
                }
            }
            TokenKind::OpenPrnth => {
                self.assert_next(TokenKind::OpenPrnth)?;
                let idx0 = self.parse_index()?;
                self.assert_next(TokenKind::Comma)?;
                let idx1 = self.parse_index()?;
                self.assert_next(TokenKind::ClosePrnth)?;

                Ok((idx0, idx1))
            }
            _ => Err(ParseError {
                pos: token.pos,
                cause: Box::new(ParseErrorCause::UnexpectedToken(
                    self.input.next().unwrap().unwrap(),
                )),
            }),
        }
    }

    fn parse_pipes(&mut self) -> Result<Vec<Stars>> {
        let mut pipes = Vec::new();

        loop {
            let stars = self.parse_stars()?;
            pipes.push(stars);

            let token = match self.input.peek() {
                Some(token) => token,
                None => return Ok(pipes),
            }?;

            match token.kind {
                TokenKind::Pipe => {
                    self.assert_next(TokenKind::Pipe)?;
                    continue;
                }
                _ => return Ok(pipes),
            }
        }
    }

    fn parse_stars(&mut self) -> Result<Stars> {
        let mut stars = Vec::new();

        #[derive(PartialEq)]
        enum LookingFor {
            Any,
            Star,
            Key,
        }

        let mut looking_for = LookingFor::Any;

        loop {
            let token = match self.input.peek() {
                Some(token) => token,
                None => {
                    if looking_for != LookingFor::Star {
                        stars.push(String::new());
                    }

                    return Ok(Stars(stars));
                }
            }?;

            match &token.kind {
                TokenKind::Key(_) => {
                    if looking_for == LookingFor::Star {
                        return Err(ParseError {
                            pos: token.pos,
                            cause: Box::new(ParseErrorCause::UnexpectedToken(
                                self.input.next().unwrap().unwrap(),
                            )),
                        });
                    }
                    let token = self.input.next().unwrap().unwrap();

                    looking_for = LookingFor::Star;

                    match token.kind {
                        TokenKind::Key(key) => stars.push(key),
                        _ => unreachable!(),
                    }
                }
                TokenKind::Star => {
                    if looking_for == LookingFor::Key {
                        return Err(ParseError {
                            pos: token.pos,
                            cause: Box::new(ParseErrorCause::UnexpectedToken(
                                self.input.next().unwrap().unwrap(),
                            )),
                        });
                    }

                    if looking_for == LookingFor::Any {
                        stars.push(String::new());
                    }

                    self.assert_next(TokenKind::Star)?;

                    looking_for = LookingFor::Key;
                }
                _ => {
                    if looking_for != LookingFor::Star {
                        stars.push(String::new());
                    }

                    return Ok(Stars(stars));
                }
            }
        }
    }

    fn parse_index(&mut self) -> Result<usize> {
        let token = self.input.next().ok_or(ParseError {
            pos: self.input.pos(),
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })??;

        match token.kind {
            TokenKind::Key(key) => key.parse().map_err(|e| ParseError {
                pos: token.pos,
                cause: Box::new(ParseErrorCause::InvalidIndex(e)),
            }),
            _ => Err(ParseError {
                pos: token.pos,
                cause: Box::new(ParseErrorCause::UnexpectedToken(token)),
            }),
        }
    }
}
