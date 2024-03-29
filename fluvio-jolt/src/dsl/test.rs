use super::ast::{Rhs, Lhs, Stars, IndexOp, RhsEntry, RhsPart};

struct LhsTestCase<'a> {
    expr: &'a str,
    expected: Lhs,
}

impl<'a> LhsTestCase<'a> {
    pub fn run(&self) {
        let output = match Lhs::parse(self.expr) {
            Ok(output) => output,
            Err(e) => panic!("\nfailed to parse {} as lhs:\n{}", self.expr, e),
        };

        if output != self.expected {
            panic!(
                "\nwhen parsing lhs.\nexpected={:#?}\ngot={:#?}",
                self.expected, output
            );
        }
    }
}

#[test]
fn test_parse_lhs_at_empty() {
    LhsTestCase {
        expr: "@",
        expected: Lhs::At(0, Rhs(vec![]).into()),
    }
    .run();
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
        expected: Lhs::Literal("my123 _12\n3key".into()),
    }
    .run();
}

#[test]
fn test_parse_lhs_star() {
    LhsTestCase {
        expr: "*",
        expected: Lhs::Pipes(vec![Stars(vec!["".into(), "".into()])]),
    }
    .run();
}

#[test]
fn test_parse_lhs_stars() {
    LhsTestCase {
        expr: "qwe*asd*zxc",
        expected: Lhs::Pipes(vec![Stars(vec!["qwe".into(), "asd".into(), "zxc".into()])]),
    }
    .run();
}

#[test]
fn test_parse_lhs_stars_leading() {
    LhsTestCase {
        expr: "*qwe*asd*zxc",
        expected: Lhs::Pipes(vec![Stars(vec![
            "".into(),
            "qwe".into(),
            "asd".into(),
            "zxc".into(),
        ])]),
    }
    .run();
}

#[test]
fn test_parse_lhs_stars_trailing() {
    LhsTestCase {
        expr: "qwe*asd*zxc*",
        expected: Lhs::Pipes(vec![Stars(vec![
            "qwe".into(),
            "asd".into(),
            "zxc".into(),
            "".into(),
        ])]),
    }
    .run();
}

#[test]
fn test_parse_lhs_pipe() {
    LhsTestCase {
        expr: "qwe|asd|zxc",
        expected: Lhs::Pipes(vec![
            Stars(vec!["qwe".into()]),
            Stars(vec!["asd".into()]),
            Stars(vec!["zxc".into()]),
        ]),
    }
    .run();
}

#[test]
fn test_parse_lhs_pipe_trailing() {
    LhsTestCase {
        expr: "qwe|asd|zxc|",
        expected: Lhs::Pipes(vec![
            Stars(vec!["qwe".into()]),
            Stars(vec!["asd".into()]),
            Stars(vec!["zxc".into()]),
            Stars(vec!["".into()]),
        ]),
    }
    .run();
}

#[test]
fn test_parse_lhs_pipe_leading() {
    LhsTestCase {
        expr: "|qwe|asd|zxc",
        expected: Lhs::Pipes(vec![
            Stars(vec!["".into()]),
            Stars(vec!["qwe".into()]),
            Stars(vec!["asd".into()]),
            Stars(vec!["zxc".into()]),
        ]),
    }
    .run();
}

#[test]
fn test_parse_lhs_at_full() {
    LhsTestCase {
        expr: "@(0,qwe)",
        expected: Lhs::At(0, "qwe".into()),
    }
    .run();
}

#[test]
fn test_parse_lhs_at_short() {
    LhsTestCase {
        expr: "@",
        expected: Lhs::At(0, Rhs(Vec::new()).into()),
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
        expr: "&(12)",
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
        expr: "$(15)",
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

#[test]
fn test_parse_lhs_misc() {
    LhsTestCase {
        expr: "@(2,clone&(1,1)_GCPerProIdenInfoPhyInfoStreet)",
        expected: Lhs::At(
            2,
            Box::new(Rhs(vec![RhsPart::CompositeKey(vec![
                RhsEntry::Key("clone".into()),
                RhsEntry::Amp(1, 1),
                RhsEntry::Key("_GCPerProIdenInfoPhyInfoStreet".into()),
            ])])),
        ),
    }
    .run();
    LhsTestCase {
        expr: "@(3,foo\\.bar)",
        expected: Lhs::At(
            3,
            Box::new(Rhs(vec![RhsPart::Key(RhsEntry::Key("foo.bar".into()))])),
        ),
    }
    .run();
    LhsTestCase {
        expr: "@(clientNameStuff.clientName)",
        expected: Lhs::At(
            0,
            Box::new(Rhs(vec![
                RhsPart::Key(RhsEntry::Key("clientNameStuff".into())),
                RhsPart::Key(RhsEntry::Key("clientName".into())),
            ])),
        ),
    }
    .run();
}

struct RhsTestCase<'a> {
    expr: &'a str,
    expected: Rhs,
}

impl<'a> RhsTestCase<'a> {
    pub fn run(&self) {
        let output = match Rhs::parse(self.expr) {
            Ok(output) => output,
            Err(e) => panic!("\nfailed to parse {} as rhs:\n{}", self.expr, e),
        };

        if output != self.expected {
            panic!(
                "\nwhen parsing rhs.\nexpected={:#?}\ngot={:#?}",
                self.expected, output
            );
        }
    }
}

impl From<&str> for Box<Rhs> {
    fn from(s: &str) -> Box<Rhs> {
        Box::new(Rhs(vec![RhsPart::Key(RhsEntry::Key(s.into()))]))
    }
}

#[test]
fn test_parse_rhs_amp_short() {
    RhsTestCase {
        expr: "&",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Amp(0, 0))]),
    }
    .run();
}

#[test]
fn test_parse_rhs_amp_short_troll() {
    RhsTestCase {
        expr: "&(12)",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Amp(12, 0))]),
    }
    .run();
}

#[test]
fn test_parse_rhs_at_full() {
    RhsTestCase {
        expr: "@(0,qwe)",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::At(0, "qwe".into()))]),
    }
    .run();
}

#[test]
fn test_parse_rhs_at_idx_amp() {
    RhsTestCase {
        expr: "@(0,qwe)[&(1,2)]",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::At(0, "qwe".into())),
            RhsPart::Index(IndexOp::Amp(1, 2)),
        ]),
    }
    .run();
}

#[test]
fn test_parse_rhs_at_idx_lit() {
    RhsTestCase {
        expr: "@(0,qwe)[27]",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::At(0, "qwe".into())),
            RhsPart::Index(IndexOp::Literal(27)),
        ]),
    }
    .run();
}

#[test]
fn test_parse_rhs_key() {
    RhsTestCase {
        expr: "hello.world",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("hello".into())),
            RhsPart::Key(RhsEntry::Key("world".into())),
        ]),
    }
    .run();
}

#[test]
fn test_parse_rhs_key_idx_lit() {
    RhsTestCase {
        expr: "hello.world[13]",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("hello".into())),
            RhsPart::Key(RhsEntry::Key("world".into())),
            RhsPart::Index(IndexOp::Literal(13)),
        ]),
    }
    .run();
}

#[test]
fn test_parse_rhs_misc() {
    RhsTestCase {
        expr: "photos[&(1)].id",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("photos".into())),
            RhsPart::Index(IndexOp::Amp(1, 0)),
            RhsPart::Key(RhsEntry::Key("id".into())),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "photos[&(3)].sizes.&(1)",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("photos".into())),
            RhsPart::Index(IndexOp::Amp(3, 0)),
            RhsPart::Key(RhsEntry::Key("sizes".into())),
            RhsPart::Key(RhsEntry::Amp(1, 0)),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "This is a review",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Key("This is a review".into()))]),
    }
    .run();
    RhsTestCase {
        expr: "rating-&",
        expected: Rhs(vec![RhsPart::CompositeKey(vec![
            RhsEntry::Key("rating-".into()),
            RhsEntry::Amp(0, 0),
        ])]),
    }
    .run();
    RhsTestCase {
        expr: "listOfFooValues[]",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("listOfFooValues".into())),
            RhsPart::Index(IndexOp::Empty),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "sillyListOfTunaIds[].id",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("sillyListOfTunaIds".into())),
            RhsPart::Index(IndexOp::Empty),
            RhsPart::Key(RhsEntry::Key("id".into())),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "photo-&-url",
        expected: Rhs(vec![RhsPart::CompositeKey(vec![
            RhsEntry::Key("photo-".into()),
            RhsEntry::Amp(0, 0),
            RhsEntry::Key("-url".into()),
        ])]),
    }
    .run();
    RhsTestCase {
        expr: "9876",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Key("9876".into()))]),
    }
    .run();
    RhsTestCase {
        expr: "&[]",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Amp(0, 0)),
            RhsPart::Index(IndexOp::Empty),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "clients.@(3,clientId)",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("clients".into())),
            RhsPart::Key(RhsEntry::At(3, "clientId".into())),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "&(1).&(3)[]",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Amp(1, 0)),
            RhsPart::Key(RhsEntry::Amp(3, 0)),
            RhsPart::Index(IndexOp::Empty),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "&",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Amp(0, 0))]),
    }
    .run();
    RhsTestCase {
        expr: "sillyPhotoData.@(captions[1])",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("sillyPhotoData".into())),
            RhsPart::Key(RhsEntry::At(
                0,
                Box::new(Rhs(vec![
                    RhsPart::Key(RhsEntry::Key("captions".into())),
                    RhsPart::Index(IndexOp::Literal(1)),
                ])),
            )),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "states.@(2,states[&])",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("states".into())),
            RhsPart::Key(RhsEntry::At(
                2,
                Box::new(Rhs(vec![
                    RhsPart::Key(RhsEntry::Key("states".into())),
                    RhsPart::Index(IndexOp::Amp(0, 0)),
                ])),
            )),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "ratings.&(1).label",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("ratings".into())),
            RhsPart::Key(RhsEntry::Amp(1, 0)),
            RhsPart::Key(RhsEntry::Key("label".into())),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "@(clientNameStuff).clientName",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::At(0, "clientNameStuff".into())),
            RhsPart::Key(RhsEntry::Key("clientName".into())),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "[&(1)].guid",
        expected: Rhs(vec![
            RhsPart::Index(IndexOp::Amp(1, 0)),
            RhsPart::Key(RhsEntry::Key("guid".into())),
        ]),
    }
    .run();
    RhsTestCase {
        expr: "__data.&",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("__data".into())),
            RhsPart::Key(RhsEntry::Amp(0, 0)),
        ]),
    }
    .run();
}

#[test]
fn test_parse_rhs_escape() {
    RhsTestCase {
        expr: "\\@A",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Key("@A".into()))]),
    }
    .run();
    RhsTestCase {
        expr: "\\$B",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Key("$B".into()))]),
    }
    .run();
    RhsTestCase {
        expr: "\\&C",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Key("&C".into()))]),
    }
    .run();
    RhsTestCase {
        expr: "\\[D",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Key("[D".into()))]),
    }
    .run();
    RhsTestCase {
        expr: "\\[\\]E",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Key("[]E".into()))]),
    }
    .run();
    RhsTestCase {
        expr: "\\]F",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Key("]F".into()))]),
    }
    .run();
    RhsTestCase {
        expr: "\\*G",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Key("*G".into()))]),
    }
    .run();
    RhsTestCase {
        expr: "\\#H",
        expected: Rhs(vec![RhsPart::Key(RhsEntry::Key("#H".into()))]),
    }
    .run();
}

#[test]
fn test_parse_rhs_empty() {
    RhsTestCase {
        expr: "",
        expected: Rhs(vec![]),
    }
    .run();
}

#[test]
fn test_parse_rhs_idx_at() {
    RhsTestCase {
        expr: "hello[@(2,world)]",
        expected: Rhs(vec![
            RhsPart::Key(RhsEntry::Key("hello".into())),
            RhsPart::Index(IndexOp::At(2, "world".into())),
        ]),
    }
    .run();
}
