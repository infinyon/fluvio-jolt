use super::error::{ParseErrorCause, ParseError};
use super::token::{Token, TokenKind};
use super::tokenizer::Tokenizer;
use std::result::Result as StdResult;
use super::ast::{Lhs, Rhs, IndexOp, RhsEntry, Stars, RhsPart};

const MAX_DEPTH: usize = 4;

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
        let token = match self.input.next()? {
            Some(token) => token,
            None => return Ok(Lhs::Literal(String::new())),
        };

        let res = match token.kind {
            TokenKind::Square => self.parse_square_lhs().map(Lhs::Square),
            TokenKind::At => self.parse_at_tuple(0).map(|t| Lhs::At(t.0, t.1)),
            TokenKind::DollarSign => self.parse_num_tuple().map(|t| Lhs::DollarSign(t.0, t.1)),
            TokenKind::Amp => self.parse_num_tuple().map(|t| Lhs::Amp(t.0, t.1)),
            TokenKind::Key(_) | TokenKind::Star => {
                self.input.put_back(token);
                self.parse_pipes_or_lit()
            }
            _ => {
                return Err(ParseError {
                    pos: token.pos,
                    cause: Box::new(ParseErrorCause::UnexpectedToken(token)),
                });
            }
        }?;

        if let Some(token) = self.input.next()? {
            return Err(ParseError {
                pos: token.pos,
                cause: Box::new(ParseErrorCause::UnexpectedToken(token)),
            });
        }

        Ok(res)
    }

    pub fn parse_rhs(&mut self) -> Result<Rhs> {
        let rhs = self.parse_rhs_impl(0)?;

        if let Some(token) = self.input.next()? {
            return Err(ParseError {
                pos: token.pos,
                cause: Box::new(ParseErrorCause::UnexpectedToken(token)),
            });
        }

        Ok(rhs)
    }

    fn parse_rhs_impl(&mut self, depth: usize) -> Result<Rhs> {
        if depth > MAX_DEPTH {
            return Err(ParseError {
                pos: self.input.pos(),
                cause: ParseErrorCause::MaximumRecursion(MAX_DEPTH).into(),
            });
        }

        let mut parts = Vec::new();

        let token = match self.input.next()? {
            Some(token) => token,
            None => return Ok(Rhs(parts)),
        };

        match token.kind {
            TokenKind::OpenBrkt => {
                let idx_op = self.parse_index_op(depth)?;
                self.assert_next(TokenKind::CloseBrkt)?;
                parts.push(RhsPart::Index(idx_op));
            }
            _ => {
                self.input.put_back(token);
                parts.push(self.parse_rhs_part(depth)?);
            }
        }

        while let Some(token) = self.input.next()? {
            match token.kind {
                TokenKind::OpenBrkt => {
                    let idx_op = self.parse_index_op(depth)?;
                    self.assert_next(TokenKind::CloseBrkt)?;
                    parts.push(RhsPart::Index(idx_op));
                }
                TokenKind::Dot => {
                    parts.push(self.parse_rhs_part(depth)?);
                }
                _ => {
                    self.input.put_back(token)?;
                    break;
                }
            }
        }

        Ok(Rhs(parts))
    }

    fn parse_rhs_part(&mut self, depth: usize) -> Result<RhsPart> {
        let mut entries: Vec<RhsEntry> = Vec::new();

        while let Some(token) = self.input.next()? {
            let res = match token.kind {
                TokenKind::Amp => self.parse_num_tuple().map(|t| RhsEntry::Amp(t.0, t.1))?,
                TokenKind::At => self.parse_at_tuple(depth).map(|t| RhsEntry::At(t.0, t.1))?,
                TokenKind::Key(key) => RhsEntry::Key(key),
                _ => {
                    self.input.put_back(token)?;
                    break;
                }
            };

            entries.push(res);
        }

        let part = match entries.len() {
            0 => RhsPart::Key(RhsEntry::Key(String::new())),
            1 => RhsPart::Key(entries.remove(0)),
            _ => RhsPart::CompositeKey(entries),
        };

        Ok(part)
    }

    fn parse_index_op(&mut self, depth: usize) -> Result<IndexOp> {
        let token = self.get_next()?;

        let op = match token.kind {
            TokenKind::Amp => {
                let t = self.parse_num_tuple()?;
                IndexOp::Amp(t.0, t.1)
            }
            TokenKind::CloseBrkt => {
                self.input.put_back(token)?;
                IndexOp::Empty
            }
            TokenKind::Key(key) => IndexOp::Literal(Self::parse_index(&key, token.pos)?),
            TokenKind::At => {
                let t = self.parse_at_tuple(depth)?;
                IndexOp::At(t.0, t.1)
            }
            _ => {
                return Err(ParseError {
                    pos: token.pos,
                    cause: Box::new(ParseErrorCause::UnexpectedToken(token)),
                });
            }
        };

        Ok(op)
    }

    fn parse_square_lhs(&mut self) -> Result<String> {
        let token = match self.input.next()? {
            Some(token) => token,
            None => return Ok(String::new()),
        };

        match token.kind {
            TokenKind::Key(key) => Ok(key),
            _ => Err(ParseError {
                pos: token.pos,
                cause: Box::new(ParseErrorCause::UnexpectedToken(token)),
            }),
        }
    }

    fn parse_at_tuple(&mut self, depth: usize) -> Result<(usize, Box<Rhs>)> {
        let token = match self.input.next()? {
            Some(token) => token,
            None => return Ok((0, Rhs(Vec::new()).into())),
        };

        if token.kind != TokenKind::OpenPrnth {
            self.input.put_back(token)?;
            return Ok((0, Rhs(Vec::new()).into()));
        }

        let rhs_pos = self.input.pos();
        let rhs = self.parse_rhs_impl(depth + 1)?;

        let token = self.get_next()?;

        let idx = match token.kind {
            TokenKind::Comma => Self::rhs_to_idx(rhs, rhs_pos)?,
            TokenKind::ClosePrnth => {
                return Ok((0, rhs.into()));
            }
            _ => {
                return Err(ParseError {
                    pos: token.pos,
                    cause: ParseErrorCause::UnexpectedToken(token).into(),
                });
            }
        };

        let rhs = self.parse_rhs_impl(depth + 1)?;

        self.assert_next(TokenKind::ClosePrnth)?;

        Ok((idx, rhs.into()))
    }

    fn parse_num_tuple(&mut self) -> Result<(usize, usize)> {
        let token = match self.input.next()? {
            Some(token) => token,
            None => return Ok((0, 0)),
        };

        if token.kind != TokenKind::OpenPrnth {
            self.input.put_back(token)?;
            return Ok((0, 0));
        }

        let get_idx = || {
            let token = self.get_next()?;
            match token.kind {
                TokenKind::Key(key) => Self::parse_index(&key, token.pos),
                _ => Err(ParseError {
                    pos: token.pos,
                    cause: ParseErrorCause::ExpectedIdx.into(),
                }),
            }
        };

        let idx0 = get_idx()?;

        let token = self.get_next()?;
        match token.kind {
            TokenKind::Comma => (),
            TokenKind::ClosePrnth => {
                return Ok((idx0, 0));
            }
            _ => {
                return Err(ParseError {
                    pos: token.pos,
                    cause: ParseErrorCause::UnexpectedToken(token).into(),
                })
            }
        }

        let idx1 = get_idx()?;

        self.assert_next(TokenKind::ClosePrnth);

        Ok((idx0, idx1))
    }

    fn parse_pipes_or_lit(&mut self) -> Result<Lhs> {
        let pipes = self.parse_pipes()?;

        // check if pipes is only a single string literal
        // the raw indexing will never panic since we check the lengths first
        if pipes.len() == 0 && pipes[0].0.len() == 0 {
            Ok(Lhs::Literal(pipes[0].0[0]))
        } else {
            Ok(Lhs::Pipes(pipes))
        }
    }

    fn parse_pipes(&mut self) -> Result<Vec<Stars>> {
        let mut pipes = Vec::new();

        while let Some(token) = self.input.next()? {
            match token.kind {
                TokenKind::Key(_) | TokenKind::Star => {
                    self.input.put_back(token)?;
                    pipes.push(self.parse_stars()?);

                    match self.input.next()? {
                        Some(token) => {
                            if token.kind == TokenKind::Pipe {
                                continue;
                            } else {
                                self.input.put_back(token)?;
                                break;
                            }
                        }
                        None => break,
                    }
                }
                TokenKind::Pipe => pipes.push(Stars(Vec::new())),
                _ => {
                    self.input.put_back(token)?;
                    break;
                }
            }
        }

        Ok(pipes)
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

    fn parse_index(key: &str, pos: usize) -> Result<usize> {
        key.parse().map_err(|e| ParseError {
            pos,
            cause: Box::new(ParseErrorCause::InvalidIndex(e)),
        })
    }

    fn rhs_to_idx(mut rhs: Rhs, pos: usize) -> Result<usize> {
        let key = match rhs.0.pop() {
            Some(RhsPart::Key(RhsEntry::Key(key))) if rhs.0.is_empty() => key,
            _ => {
                return Err(ParseError {
                    pos,
                    cause: ParseErrorCause::ExpectedIdx.into(),
                });
            }
        };

        Self::parse_index(&key, pos)
    }

    fn get_next(&mut self) -> Result<Token> {
        self.input.next()?.ok_or(ParseError {
            pos: self.input.pos(),
            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
        })
    }

    fn assert_next(&mut self, expected: TokenKind) -> Result<()> {
        let got = self.get_next()?;
        if expected == got.kind {
            Ok(())
        } else {
            Err(ParseError {
                pos: got.pos,
                cause: Box::new(ParseErrorCause::UnexpectedToken(got)),
            })
        }
    }
}
