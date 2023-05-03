use super::util::test_dir;

#[test]
fn test_shift_transform() {
    test_dir(
        "jolt-java/jolt-core/src/test/resources/json/shiftr",
        "shift",
        &[
            "mapToList", // skipped because not implemented yet.
            "passNullThru",
            "pollaxman_218_duplicate_speclines_bug", // skipped because we error if key not found in object in lhs expr
        ],
    );
}
