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
            None => return Ok(Lhs::Pipes(vec![Stars(vec![String::new()])])),
        };

        let res = match token.kind {
            TokenKind::Square => self.parse_square_lhs().map(Lhs::Square),
            TokenKind::At => self.parse_at_tuple(0).map(|t| Lhs::At(t.0, t.1)),
            TokenKind::DollarSign => self.parse_num_tuple().map(|t| Lhs::DollarSign(t.0, t.1)),
            TokenKind::Amp => self.parse_num_tuple().map(|t| Lhs::Amp(t.0, t.1)),
            TokenKind::Key(_) | TokenKind::Star | TokenKind::Pipe => {
                self.input.put_back(token)?;
                self.parse_pipes_or_lit()
            }
            TokenKind::Eq => self.parse_fn_call(0).map(|t| Lhs::FnCall(t.0, t.1)),
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
                self.input.put_back(token)?;
                if let Some(part) = self.parse_rhs_part(depth)? {
                    parts.push(part);
                }
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
                    if let Some(part) = self.parse_rhs_part(depth)? {
                        parts.push(part);
                    } else {
                        break;
                    }
                }
                _ => {
                    self.input.put_back(token)?;
                    break;
                }
            }
        }

        Ok(Rhs(parts))
    }

    fn parse_rhs_part(&mut self, depth: usize) -> Result<Option<RhsPart>> {
        let mut entries: Vec<RhsEntry> = Vec::new();

        while let Some(token) = self.input.next()? {
            let res = match token.kind {
                TokenKind::Amp => self.parse_num_tuple().map(|t| RhsEntry::Amp(t.0, t.1))?,
                TokenKind::At => self.parse_at_tuple(depth).map(|t| RhsEntry::At(t.0, t.1))?,
                TokenKind::Key(key) => RhsEntry::Key(key),
                TokenKind::Eq => self
                    .parse_fn_call(depth + 1)
                    .map(|t| RhsEntry::FnCall(t.0, t.1))?,
                _ => {
                    self.input.put_back(token)?;
                    break;
                }
            };

            entries.push(res);
        }

        let part = match entries.len() {
            0 => return Ok(None),
            1 => RhsPart::Key(entries.remove(0)),
            _ => RhsPart::CompositeKey(entries),
        };

        Ok(Some(part))
    }

    fn parse_fn_call(&mut self, depth: usize) -> Result<(String, Vec<Rhs>)> {
        let token = self.get_next()?;
        let name = match token.kind {
            TokenKind::Key(name) => name,
            _ => {
                return Err(ParseError {
                    pos: token.pos,
                    cause: ParseErrorCause::UnexpectedToken(token).into(),
                })
            }
        };

        let mut args = Vec::new();

        self.assert_next(TokenKind::OpenPrnth);
        let token = self.get_next()?;
        match token.kind {
            TokenKind::ClosePrnth => {
                return Ok((name, args));
            }
            _ => {
                self.input.put_back(token)?;
            }
        }

        args.push(self.parse_rhs_impl(depth + 1)?);

        while let Some(token) = self.input.next()? {
            match token.kind {
                TokenKind::Comma => {
                    args.push(self.parse_rhs_impl(depth + 1)?);
                }
                TokenKind::ClosePrnth => {
                    return Ok((name, args));
                }
                _ => {
                    return Err(ParseError {
                        pos: token.pos,
                        cause: ParseErrorCause::UnexpectedToken(token).into(),
                    })
                }
            }
        }

        Ok((name, args))
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

        let get_idx = |p: &mut Self| {
            let token = p.get_next()?;
            match token.kind {
                TokenKind::Key(key) => Self::parse_index(&key, token.pos),
                _ => Err(ParseError {
                    pos: token.pos,
                    cause: ParseErrorCause::ExpectedIdx.into(),
                }),
            }
        };

        let idx0 = get_idx(self)?;

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

        let idx1 = get_idx(self)?;

        self.assert_next(TokenKind::ClosePrnth)?;

        Ok((idx0, idx1))
    }

    fn parse_pipes_or_lit(&mut self) -> Result<Lhs> {
        let pipes = self.parse_pipes()?;

        if pipes.len() == 1 && pipes[0].0.len() == 1 {
            // this will never panic because we check the lengths
            // beforehand
            let mut pipes = pipes;
            Ok(Lhs::Literal(pipes.pop().unwrap().0.pop().unwrap()))
        } else {
            Ok(Lhs::Pipes(pipes))
        }
    }

    fn parse_pipes(&mut self) -> Result<Vec<Stars>> {
        let mut pipes = Vec::new();

        #[derive(PartialEq)]
        enum Last {
            None,
            Stars,
            Pipe,
        }

        let mut last = Last::None;

        while let Some(token) = self.input.next()? {
            match token.kind {
                TokenKind::Key(_) | TokenKind::Star => {
                    match last {
                        Last::None | Last::Pipe => {
                            self.input.put_back(token)?;
                            pipes.push(self.parse_stars()?);
                        }
                        Last::Stars => {
                            return Err(ParseError {
                                pos: token.pos,
                                cause: ParseErrorCause::UnexpectedToken(token).into(),
                            })
                        }
                    }

                    last = Last::Stars;
                }
                TokenKind::Pipe => {
                    match last {
                        Last::None => pipes.push(Stars(vec![String::new()])),
                        Last::Stars => (),
                        Last::Pipe => {
                            return Err(ParseError {
                                pos: token.pos,
                                cause: ParseErrorCause::UnexpectedToken(token).into(),
                            })
                        }
                    }

                    last = Last::Pipe;
                }
                _ => {
                    self.input.put_back(token)?;
                    break;
                }
            }
        }

        if last == Last::Pipe {
            pipes.push(Stars(vec![String::new()]));
        }

        Ok(pipes)
    }

    fn parse_stars(&mut self) -> Result<Stars> {
        let mut stars = Vec::new();

        #[derive(PartialEq)]
        enum Last {
            None,
            Star,
            Key,
        }

        let mut last = Last::None;

        while let Some(token) = self.input.next()? {
            match token.kind {
                TokenKind::Star => {
                    match last {
                        Last::None => stars.push(String::new()),
                        Last::Star => {
                            return Err(ParseError {
                                pos: token.pos,
                                cause: ParseErrorCause::UnexpectedToken(token).into(),
                            })
                        }
                        Last::Key => (),
                    }

                    last = Last::Star;
                }
                TokenKind::Key(key) => {
                    match last {
                        Last::None | Last::Star => stars.push(key),
                        Last::Key => {
                            return Err(ParseError {
                                pos: token.pos,
                                cause: ParseErrorCause::UnexpectedToken(Token {
                                    pos: token.pos,
                                    kind: TokenKind::Key(key),
                                })
                                .into(),
                            })
                        }
                    }

                    last = Last::Key;
                }
                _ => {
                    self.input.put_back(token)?;
                    break;
                }
            }
        }

        if last == Last::Star {
            stars.push(String::new());
        }

        Ok(Stars(stars))
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
