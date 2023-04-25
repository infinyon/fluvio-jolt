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

mod rhs_tests {
    use super::*;

    struct RhsTestCase<'a> {
        expr: &'a str,
        expected: Rhs,
    }

    impl<'a> RhsTestCase<'a> {
        pub fn run(&self) {
            let output = match Rhs::parse(self.expr) {
                Ok(output) => output,
                Err(e) => panic!("failed to parse {} as rhs:\n{}", self.expr, e),
            };

            if output != self.expected {
                panic!(
                    "when parsing rhs.\nexpected={:#?}\ngot={:#?}",
                    self.expected, output
                );
            }
        }
    }

    #[test]
    fn test_parse_rhs_amp_short() {
        RhsTestCase {
            expr: "&",
            expected: Rhs(vec![RhsEntry::Amp(0, 0)]),
        }
        .run();
    }

    #[test]
    fn test_parse_rhs_at_full() {
        RhsTestCase {
            expr: "@(0,qwe)",
            expected: Rhs(vec![RhsEntry::At(Some((0, "qwe".into())))]),
        }
        .run();
    }

    #[test]
    fn test_parse_rhs_at_idx_square() {
        RhsTestCase {
            expr: "@(0,qwe)[#15]",
            expected: Rhs(vec![
                RhsEntry::At(Some((0, "qwe".into()))),
                RhsEntry::Index(IndexOp::Square(15)),
            ]),
        }
        .run();
    }

    #[test]
    fn test_parse_rhs_at_idx_amp() {
        RhsTestCase {
            expr: "@(0,qwe)[&(1,2)]",
            expected: Rhs(vec![
                RhsEntry::At(Some((0, "qwe".into()))),
                RhsEntry::Index(IndexOp::Amp(1, 2)),
            ]),
        }
        .run();
    }

    #[test]
    fn test_parse_rhs_at_idx_lit() {
        RhsTestCase {
            expr: "@(0,qwe)[27]",
            expected: Rhs(vec![
                RhsEntry::At(Some((0, "qwe".into()))),
                RhsEntry::Index(IndexOp::Literal(27)),
            ]),
        }
        .run();
    }

    #[test]
    fn test_parse_rhs_key() {
        RhsTestCase {
            expr: "hello.world",
            expected: Rhs(vec![
                RhsEntry::Key("hello".into()),
                RhsEntry::Dot,
                RhsEntry::Key("world".into()),
            ]),
        }
        .run();
    }

    #[test]
    fn test_parse_rhs_key_idx_lit() {
        RhsTestCase {
            expr: "hello.world[13]",
            expected: Rhs(vec![
                RhsEntry::Key("hello".into()),
                RhsEntry::Dot,
                RhsEntry::Key("world".into()),
                RhsEntry::Index(IndexOp::Literal(13)),
            ]),
        }
        .run();
    }

    #[test]
    fn test_parse_rhs_misc() {
        RhsTestCase {
            expr: "photos[&1].id",
            expected: Rhs(vec![
                RhsEntry::Key("photos".into()),
                RhsEntry::Index(IndexOp::Amp(1, 0)),
                RhsEntry::Dot,
                RhsEntry::Key("id".into()),
            ]),
        }
        .run();
        RhsTestCase {
            expr: "photos[&3].sizes.&1",
            expected: Rhs(vec![
                RhsEntry::Key("photos".into()),
                RhsEntry::Index(IndexOp::Amp(3, 0)),
                RhsEntry::Dot,
                RhsEntry::Key("sizes".into()),
                RhsEntry::Dot,
                RhsEntry::Amp(1, 0),
            ]),
        }
        .run();
        RhsTestCase {
            expr: "This is a review",
            expected: Rhs(vec![RhsEntry::Key("This is a review".into())]),
        }
        .run();
        RhsTestCase {
            expr: "rating-&",
            expected: Rhs(vec![RhsEntry::Key("rating-".into()), RhsEntry::Amp(0, 0)]),
        }
        .run();
        RhsTestCase {
            expr: "listOfFooValues[]",
            expected: Rhs(vec![
                RhsEntry::Key("listOfFooValues".into()),
                RhsEntry::Index(IndexOp::Empty),
            ]),
        }
        .run();
        RhsTestCase {
            expr: "sillyListOfTunaIds[].id",
            expected: Rhs(vec![
                RhsEntry::Key("sillyListOfTunaIds".into()),
                RhsEntry::Index(IndexOp::Empty),
                RhsEntry::Dot,
                RhsEntry::Key("id".into()),
            ]),
        }
        .run();
    }
}
