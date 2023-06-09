use super::util::test_dir;

#[test]
fn test_shift_transform() {
    test_dir("tests/java/resources/shift", "shift", &[]);

    test_dir("tests/data/shift", "shift", &[]);
}
