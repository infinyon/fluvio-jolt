use super::parser::Parser;
use super::ParseError;

#[derive(Debug, PartialEq)]
pub enum Lhs {
    DollarSign(usize, usize),
    Amp(usize, usize),
    At(Option<(usize, String)>),
    Square(String),
    Key(KeySelection),
}

impl Lhs {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
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
pub struct Rhs(pub Vec<RhsEntry>);

#[derive(Debug, PartialEq)]
pub enum RhsEntry {
    Amp(usize, usize),
    At(Option<(usize, String)>),
    Index(IndexOp),
    Key(String),
    Dot,
}

#[derive(Debug, PartialEq)]
pub enum IndexOp {
    Square(usize),
    Amp(usize, usize),
    Literal(usize),
    Empty,
}

impl Rhs {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Parser::new(input).parse_rhs()
    }
}
