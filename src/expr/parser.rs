use lalrpop_util::lalrpop_mod;

lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(dead_code)]
    pub grammar
);

pub use grammar::*;

#[cfg(test)]
mod lhs_tests {
    use super::*;
    use crate::expr::ast::Lhs;

    struct LhsTestCase<'a> {
        expr: &'a str,
        expected: Vec<Lhs>,
    }

    impl<'a> LhsTestCase<'a> {
        pub fn run(&self) {
            let output = match LhsPipeParser::new().parse(self.expr) {
                Ok(output) => output,
                Err(e) => panic!("failed to parse {} as lhs\n{}", self.expr, e),
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
            expected: vec![Lhs::Square("my123 _12\n3key".into())],
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_dollar_sign() {
        LhsTestCase {
            expr: "$",
            expected: vec![Lhs::DollarSign],
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_key() {
        LhsTestCase {
            expr: "my123 _12\n3key",
            expected: vec![Lhs::Key("my123 _12\n3key".into())],
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_star() {
        LhsTestCase {
            expr: "*",
            expected: vec![Lhs::Star],
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_star_pipe() {
        LhsTestCase {
            expr: "qwe*|*qwe*|*qwe",
            expected: vec![
                Lhs::RightStar("qwe".into()),
                Lhs::BothStar("qwe".into()),
                Lhs::LeftStar("qwe".into()),
            ],
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_at_full() {
        LhsTestCase {
            expr: "@(0,qwe)",
            expected: vec![Lhs::At(Some((0, "qwe".into())))],
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_at_short() {
        LhsTestCase {
            expr: "@",
            expected: vec![Lhs::At(None)],
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_amp_short() {
        LhsTestCase {
            expr: "&",
            expected: vec![Lhs::Amp(0, 0)],
        }
        .run();
    }

    #[test]
    fn test_parse_lhs_amp_full() {
        LhsTestCase {
            expr: "&(0,1)",
            expected: vec![Lhs::Amp(0, 1)],
        }
        .run();
    }
}
