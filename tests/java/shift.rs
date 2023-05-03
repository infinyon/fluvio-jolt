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
            "simpleRHSEscape", // skipped because we don't support uncommented . in LHS
            "transposeComplex9_lookup_an_array_index", // skipped because we don't support negative indexes
            "transposeInverseMap2",                    // not supported
            "transposeNestedLookup",                   // not supported
            "wildcards.json", // skipped because we don't support uncommented . in LHS
            "wildcardSelfAndRef", // skipped because this test seems wrong
            "wildcardsWithOr", // skipped because has some weirdness with alphabetical ordering
        ],
    );

    test_dir("tests/data/shift", "shift", &[]);
}
