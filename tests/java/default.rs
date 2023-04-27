use super::util::test_dir;

#[test]
#[ignore]
fn test_default_transform() {
    test_dir(
        "jolt-java/jolt-core/src/test/resources/json/defaultr",
        "default",
    );
}
