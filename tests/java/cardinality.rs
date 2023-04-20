use super::util::test_dir;

#[test]
fn test_cardinality_transform() {
    test_dir(
        "jolt-java/jolt-core/src/test/resources/json/cardinality",
        "cardinality",
    );
}
