use super::parser::Parser;
use super::ParseError;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Lhs {
    DollarSign(usize, usize),
    Amp(usize, usize),
    At(Option<(usize, Box<Rhs>)>),
    Square(String),
    Pipes(Vec<Stars>),
}

impl Lhs {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Parser::new(input).parse_lhs()
    }
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Stars(pub Vec<String>);

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Rhs(pub Vec<RhsEntry>);

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum RhsEntry {
    Amp(usize, usize),
    At(Option<(usize, Box<Rhs>)>),
    Index(IndexOp),
    Key(String),
    Dot,
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum IndexOp {
    Square(usize),
    Amp(usize, usize),
    Literal(usize),
    At(Option<(usize, Box<Rhs>)>),
    Empty,
}

impl Rhs {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Parser::new(input).parse_rhs()
    }
}
