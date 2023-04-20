use super::util::test_dir;

#[test]
fn test_remove_transform() {
    test_dir(
        "jolt-java/jolt-core/src/test/resources/json/removr",
        "remove",
    );
}
