use std::fs::File;
use serde_json::Value;
use serde::Serialize;
use serde::Deserialize;
use fluvio_jolt::TransformSpec;

#[derive(Debug, Serialize, Deserialize)]
struct TestData {
    input: Value,
    spec: TransformSpec,
    expected: Value,
}

#[test]
fn test_all() {
    let tests = ["simple", "shift_and_default"];
    for name in tests {
        do_test(name);
    }
}

fn do_test(name: &str) {
    //given
    let file = File::open(format!(
        "{}/tests/data/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        name
    ))
    .unwrap_or_else(|_| panic!("existing file for test `{}`", name));
    let TestData {
        input,
        spec,
        expected,
    } = serde_json::from_reader::<_, TestData>(file)
        .unwrap_or_else(|_| panic!("existing file for test `{}`", name));

    //when
    let result = fluvio_jolt::transform(input, &spec);

    //then
    assert_eq!(result, expected, "failed assertion for test `{}`", name);
}
