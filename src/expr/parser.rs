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

    #[test]
    fn test_parse_lhs() {
        let lhs = LhsPipeParser::new().parse("qwe*|*qwe*").unwrap();

        assert_eq!(
            vec![Lhs::RightStar("qwe".into()), Lhs::BothStar("qwe".into()),],
            lhs
        );
    }
}
