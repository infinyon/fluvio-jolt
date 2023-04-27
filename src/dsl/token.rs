#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    DollarSign,
    Amp,
    At,
    Square,
    Star,
    Pipe,
    OpenBrkt,
    CloseBrkt,
    OpenPrnth,
    ClosePrnth,
    Dot,
    Comma,
    Key(String),
}
