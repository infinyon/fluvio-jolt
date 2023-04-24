use lalrpop_util::lalrpop_mod;

lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(dead_code)]
    pub grammar
);

pub use grammar::*;

#[cfg(test)]
mod tests {
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
    fn test_parse_lhs() {
        LhsTestCase {
            expr: "qwe*|*qwe*",
            expected: vec![Lhs::RightStar("qwe".into()), Lhs::BothStar("qwe".into())],
        }
        .run();

        LhsTestCase {
            expr: "&(0,1)",
            expected: vec![Lhs::Amp(0, 1)],
        }
        .run();
    }
}
