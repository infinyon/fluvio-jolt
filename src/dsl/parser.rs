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

    pub fn parse_lhs(&mut self) -> Result<Lhs> {
        let token = self.get_next()?;

        let res = match token.kind {
            TokenKind::Square => self.parse_square_lhs().map(Lhs::Square),
            TokenKind::At => self.parse_at_tuple(0).map(Lhs::At),
            TokenKind::DollarSign => self.parse_idx_tuple().map(|t| Lhs::DollarSign(t.0, t.1)),
            TokenKind::Amp => self.parse_idx_tuple().map(|t| Lhs::Amp(t.0, t.1)),
            _ => {
                self.input.put_back(token);
                self.parse_pipes()
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
                let idx_op = self.parse_idx_op(depth)?;
                self.assert_next(TokenKind::CloseBrkt)?;
                parts.push(RhsPart::Index(idx_op));
            }
            _ => {
                self.input.put_back(token);
                parts.push(self.parse_rhs_part()?);
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
                    parts.push(self.parse_rhs_part()?);
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
                TokenKind::At => self.parse_at_tuple(depth, true).map(RhsEntry::At)?,
                TokenKind::Key(key) => RhsEntry::Key(key),
                _ => {
                    self.input.put_back(token)?;
                    break;
                }
            }?;

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
                let t = self.parse_amp_tuple()?;
                IndexOp::Amp(t.0, t.1)
            }
            TokenKind::CloseBrkt => {
                self.input.put_back(token)?;
                IndexOp::Empty
            }
            TokenKind::Key(key) => IndexOp::Literal(Self::parse_index(&key, token.pos)?),
            TokenKind::At => {
                let t = self.parse_at_tuple(depth, true)?;
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

    fn parse_at(&mut self, depth: usize, return_on_dot: bool) -> Result<Option<(usize, Box<Rhs>)>> {
        let token = match self.input.peek() {
            Some(token) => token?,
            None => return Ok(None),
        };
        let mut assert_close_prnth = false;
        if token.kind == TokenKind::OpenPrnth {
            self.assert_next(TokenKind::OpenPrnth)?;
            assert_close_prnth = true;
        }

        let rhs = self.parse_rhs_impl(depth + 1, return_on_dot && !assert_close_prnth)?;

        let token = match self.input.peek() {
            Some(token) => token?,
            None => {
                if rhs.0.len() == 1 {
                    if let RhsPart::Key(RhsEntry::Key(k)) = rhs.0.get(0).unwrap() {
                        if k.chars().all(|c| c.is_ascii_digit()) {
                            let idx = k.parse().map_err(|e| ParseError {
                                pos: self.input.pos(),
                                cause: Box::new(ParseErrorCause::InvalidIndex(e)),
                            })?;

                            return Ok(Some((idx, Box::new(Rhs(Vec::new())))));
                        }
                    }
                }

                return Ok(Some((0, Box::new(rhs))));
            }
        };

        match &token.kind {
            TokenKind::ClosePrnth => {
                if assert_close_prnth {
                    self.assert_next(TokenKind::ClosePrnth)?;
                    Ok(Some((0, Box::new(rhs))))
                } else {
                    Err(ParseError {
                        pos: token.pos,
                        cause: Box::new(ParseErrorCause::UnexpectedToken(Token {
                            pos: token.pos,
                            kind: TokenKind::ClosePrnth,
                        })),
                    })
                }
            }
            TokenKind::Comma => {
                if rhs.0.len() != 1 {
                    return Err(ParseError {
                        pos: token.pos,
                        cause: Box::new(ParseErrorCause::UnexpectedToken(Token {
                            pos: token.pos,
                            kind: TokenKind::Comma,
                        })),
                    });
                }
                let mut rhs = rhs;
                let idx = match rhs.0.pop().unwrap() {
                    RhsPart::Key(RhsEntry::Key(key)) => key.parse().map_err(|e| ParseError {
                        pos: token.pos,
                        cause: Box::new(ParseErrorCause::InvalidIndex(e)),
                    })?,
                    _ => {
                        return Err(ParseError {
                            pos: token.pos,
                            cause: Box::new(ParseErrorCause::UnexpectedToken(Token {
                                pos: token.pos,
                                kind: TokenKind::Comma,
                            })),
                        });
                    }
                };
                self.assert_next(TokenKind::Comma)?;
                let rhs = self.parse_rhs_impl(depth + 1, return_on_dot && !assert_close_prnth)?;
                if assert_close_prnth {
                    self.assert_next(TokenKind::ClosePrnth)?;
                }
                Ok(Some((idx, Box::new(rhs))))
            }
            _ => {
                if assert_close_prnth {
                    Err(ParseError {
                        pos: self.input.pos(),
                        cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
                    })
                } else {
                    Ok(Some((0, Box::new(rhs))))
                }
            }
        }
    }

    fn parse_amp_or_ds(&mut self) -> Result<(usize, usize)> {
        if self.input.can_get_idx() == Some(Ok(true)) {
            let idx = self.input.get_idx();
            return Ok((idx, 0));
        }

        let token = match self.input.peek() {
            Some(token) => token,
            None => return Ok((0, 0)),
        }?;

        match &token.kind {
            TokenKind::OpenPrnth => {
                self.assert_next(TokenKind::OpenPrnth)?;
                let idx0 = self.parse_index()?;

                let token = match self.input.peek() {
                    Some(token) => token?,
                    None => {
                        return Err(ParseError {
                            pos: 0,
                            cause: Box::new(ParseErrorCause::UnexpectedEndOfInput),
                        })
                    }
                };
                if token.kind == TokenKind::ClosePrnth {
                    self.assert_next(TokenKind::ClosePrnth)?;
                    return Ok((idx0, 0));
                }

                self.assert_next(TokenKind::Comma)?;
                let idx1 = self.parse_index()?;
                self.assert_next(TokenKind::ClosePrnth)?;

                Ok((idx0, idx1))
            }
            _ => Ok((0, 0)),
        }
    }

    fn parse_pipes(&mut self) -> Result<Lhs> {
        let mut pipes = Vec::new();

        let pipes_to_lhs = |mut pipes: Vec<Stars>| {
            if pipes.len() == 1 {
                if pipes[0].0.len() == 1 {
                    Lhs::Literal(pipes[0].0.pop().unwrap())
                } else {
                    Lhs::Pipes(pipes)
                }
            } else {
                Lhs::Pipes(pipes)
            }
        };

        loop {
            let stars = self.parse_stars()?;

            pipes.push(stars);

            let token = match self.input.peek() {
                Some(token) => token,
                None => return Ok(pipes_to_lhs(pipes)),
            }?;

            match token.kind {
                TokenKind::Pipe => {
                    self.assert_next(TokenKind::Pipe)?;
                    continue;
                }
                _ => return Ok(pipes_to_lhs(pipes)),
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

    fn parse_index(key: &str, pos: usize) -> Result<usize> {
        key.parse().map_err(|e| ParseError {
            pos,
            cause: Box::new(ParseErrorCause::InvalidIndex(e)),
        })
    }
}
