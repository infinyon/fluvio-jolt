use super::util::test_dir;

#[test]
fn test_shift_transform() {
    test_dir(
        "jolt-java/jolt-core/src/test/resources/json/shiftr",
        "shift",
        &["filterParents3"],
    );
}
