pub enum Lhs {
    Star,
    LeftStar(String),
    RightStar(String),
    BothStar(String),
    Amp(usize, usize),
    DollarSign,
    Key(String),
    At(Option<(usize, String)>),
    Square(String),
}

pub enum Rhs {
    Amp(usize, usize),
    IndexLit(usize),
    IndexAmp(usize, usize),
    At(Option<(usize, String)>),
}
