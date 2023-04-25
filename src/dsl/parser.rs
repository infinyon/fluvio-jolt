use super::ParseError;
use super::token::Token;
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
        let c = self.chars.peek().ok_or(Error::EmptyExpr)?;

        let res = match c {
            '#' => self.parse_square_lhs().map(Lhs::Square),
            '@' => self.parse_at().map(Lhs::At),
            '$' => self.parse_dollar_sign().map(|t| Lhs::DollarSign(t.0, t.1)),
            '&' => self.parse_amp().map(|t| Lhs::Amp(t.0, t.1)),
            _ => self.parse_key_selection().map(Lhs::Key),
        }?;

        if let Some(c) = self.chars.next() {
            return Err(Error::UnexpectedCharacter(c));
        }

        Ok(res)
    }

    pub fn parse_rhs(&mut self) -> Result<Rhs> {
        let mut entries = Vec::new();

        while let Some(c) = self.chars.peek() {
            let res = match c {
                '&' => self.parse_amp().map(|t| RhsEntry::Amp(t.0, t.1)),
                '@' => self.parse_at().map(RhsEntry::At),
                '[' => self.parse_index_op().map(RhsEntry::Index),
                '.' => {
                    self.assert_next('.')?;
                    Ok(RhsEntry::Dot)
                }
                _ => self.parse_key().map(RhsEntry::Key),
            }?;

            entries.push(res);
        }

        Ok(Rhs(entries))
    }

    fn parse_key(&mut self) -> Result<String> {
        let mut key = String::new();

        while let Some(&c) = self.chars.peek() {
            match c {
                '&' | '@' | '[' | '.' => {
                    break;
                }
                _ => {
                    self.assert_next(c)?;
                    key.push(c);
                }
            }
        }

        Ok(key)
    }

    fn parse_index_op(&mut self) -> Result<IndexOp> {
        self.assert_next('[')?;

        let c = *self.chars.peek().ok_or(Error::UnexpectedEof)?;

        let op = match c {
            '#' => {
                self.assert_next('#')?;
                let idx = self.parse_index()?;
                IndexOp::Square(idx)
            }
            '&' => {
                let amp = self.parse_amp()?;
                IndexOp::Amp(amp.0, amp.1)
            }
            ']' => IndexOp::Empty,
            _ => {
                if c.is_ascii_digit() {
                    let idx = self.parse_index()?;
                    IndexOp::Literal(idx)
                } else {
                    return Err(Error::UnexpectedCharacter(c));
                }
            }
        };

        self.assert_next(']')?;

        Ok(op)
    }

    fn parse_square_lhs(&mut self) -> Result<String> {
        self.assert_next('#')?;

        let mut key = String::new();
        for c in self.chars.by_ref() {
            key.push(c);
        }

        Ok(key)
    }

    fn parse_at(&mut self) -> Result<Option<(usize, String)>> {
        self.assert_next('@')?;

        if self.chars.peek().is_none() {
            return Ok(None);
        }

        self.assert_next('(')?;
        let idx = self.parse_index()?;
        self.assert_next(',')?;
        let mut key = String::new();
        loop {
            let c = self.chars.next().ok_or(Error::UnexpectedEof)?;

            match c {
                ')' => break,
                _ => key.push(c),
            }
        }

        Ok(Some((idx, key)))
    }

    fn parse_dollar_sign(&mut self) -> Result<(usize, usize)> {
        self.assert_next('$')?;
        self.parse_amp_or_ds()
    }

    fn parse_amp(&mut self) -> Result<(usize, usize)> {
        self.assert_next('&')?;
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

    fn assert_next(&mut self, expected: char) -> Result<()> {
        let got = self.chars.next().ok_or(Error::UnexpectedEof)?;
        if expected == got {
            Ok(())
        } else {
            Err(Error::WrongCharacter { expected, got })
        }
    }
}
