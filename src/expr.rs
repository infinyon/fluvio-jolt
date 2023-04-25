use crate::error::ParseError as Error;
use std::str::Chars;
use std::iter::Peekable;
use std::ops::Range;
use std::mem;
use std::result::Result as StdResult;

type Result<T> = StdResult<T, Error>;

#[derive(Debug, PartialEq)]
pub enum Lhs {
    DollarSign(usize, usize),
    Amp(usize, usize),
    At(Option<(usize, String)>),
    Square(String),
    Key(KeySelection),
}

impl Lhs {
    pub fn parse(input: &str) -> Result<Self> {
        Parser::new(input).parse_lhs()
    }
}

#[derive(Debug, PartialEq)]
pub enum KeySelection {
    Star,
    Stars(Vec<String>),
    Literal(String),
    Pipe(Vec<String>),
}

#[derive(Debug, PartialEq)]
pub struct Rhs(Vec<RhsEntry>);

#[derive(Debug, PartialEq)]
pub enum RhsEntry {
    Amp(usize, usize),
    At(Option<(usize, String)>),
    Index(Index),
}

#[derive(Debug, PartialEq)]
pub enum Index {
    Square(usize),
    Amp(usize, usize),
    Literal(usize),
}

impl Rhs {
    pub fn parse(input: &str) -> Result<Self> {
        Parser::new(input).parse_rhs()
    }
}

struct Parser<'input> {
    chars: Peekable<Chars<'input>>,
}

impl<'input> Parser<'input> {
    fn new(input: &'input str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn parse_lhs(&mut self) -> Result<Lhs> {
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

    fn parse_rhs(&mut self) -> Result<Rhs> {
        todo!()
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
                        buf.push('*');
                        state = State::Pipe(buf, bufs);
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
                        buf.push('|');
                        state = State::Stars(buf, bufs);
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

#[cfg(test)]
mod lhs_tests {
    use super::*;

    struct LhsTestCase<'a> {
        expr: &'a str,
        expected: Lhs,
    }

    impl<'a> LhsTestCase<'a> {
        pub fn run(&self) {
            let output = match Lhs::parse(self.expr) {
                Ok(output) => output,
                Err(e) => panic!("failed to parse {} as lhs:\n{}", self.expr, e),
            };

            if output != self.expected {
                panic!(
                    "when parsing lhs.\nexpected={:#?}\ngot={:#?}",
                    self.expected, output
                );
            }
        }
    }

    #[test]
    fn test_parse_lhs_square() {
        LhsTestCase {
            expr: "#my123 _12\n3key",
            expected: Lhs::Square("my123 _12\n3key".into()),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_key() {
        LhsTestCase {
            expr: "my123 _12\n3key",
            expected: Lhs::Key(KeySelection::Literal("my123 _12\n3key".into())),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_star() {
        LhsTestCase {
            expr: "*",
            expected: Lhs::Key(KeySelection::Star),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_stars() {
        LhsTestCase {
            expr: "qwe*asd*zxc",
            expected: Lhs::Key(KeySelection::Stars(vec![
                "qwe".into(),
                "asd".into(),
                "zxc".into(),
            ])),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_stars_leading() {
        LhsTestCase {
            expr: "*qwe*asd*zxc",
            expected: Lhs::Key(KeySelection::Stars(vec![
                "".into(),
                "qwe".into(),
                "asd".into(),
                "zxc".into(),
            ])),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_stars_trailing() {
        LhsTestCase {
            expr: "qwe*asd*zxc*",
            expected: Lhs::Key(KeySelection::Stars(vec![
                "qwe".into(),
                "asd".into(),
                "zxc".into(),
                "".into(),
            ])),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_pipe() {
        LhsTestCase {
            expr: "qwe|asd|zxc",
            expected: Lhs::Key(KeySelection::Pipe(vec![
                "qwe".into(),
                "asd".into(),
                "zxc".into(),
            ])),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_pipe_trailing() {
        LhsTestCase {
            expr: "qwe|asd|zxc|",
            expected: Lhs::Key(KeySelection::Pipe(vec![
                "qwe".into(),
                "asd".into(),
                "zxc".into(),
                "".into(),
            ])),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_pipe_leading() {
        LhsTestCase {
            expr: "|qwe|asd|zxc",
            expected: Lhs::Key(KeySelection::Pipe(vec![
                "".into(),
                "qwe".into(),
                "asd".into(),
                "zxc".into(),
            ])),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_at_full() {
        LhsTestCase {
            expr: "@(0,qwe)",
            expected: Lhs::At(Some((0, "qwe".into()))),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_at_short() {
        LhsTestCase {
            expr: "@",
            expected: Lhs::At(None),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_amp_short() {
        LhsTestCase {
            expr: "&",
            expected: Lhs::Amp(0, 0),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_amp_medium() {
        LhsTestCase {
            expr: "&12",
            expected: Lhs::Amp(12, 0),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_amp_full() {
        LhsTestCase {
            expr: "&(110,12)",
            expected: Lhs::Amp(110, 12),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_dollar_sign_short() {
        LhsTestCase {
            expr: "$",
            expected: Lhs::DollarSign(0, 0),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_dollar_sign_medium() {
        LhsTestCase {
            expr: "$15",
            expected: Lhs::DollarSign(15, 0),
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_dollar_sign_full() {
        LhsTestCase {
            expr: "$(10,12)",
            expected: Lhs::DollarSign(10, 12),
        }
        .run();
    }
}
