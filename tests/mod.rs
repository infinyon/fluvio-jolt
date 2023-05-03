use std::fs::File;
use serde_json::Value;
use serde::Deserialize;
use fluvio_jolt::TransformSpec;

mod java;

#[derive(Debug, Deserialize)]
struct TestData {
    input: Value,
    spec: TransformSpec,
    expected: Value,
}

#[test]
fn test_all() {
    let tests = [
        "simple",
        "shift_and_default",
        "remove",
        "shift_wildcards",
        "simple_wildcards",
        "shift_with_or_condition",
        "variables",
        "from_benchmark",
        "nested_variables",
    ];
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
        .unwrap_or_else(|err| panic!("unable to parse file for test `{}`: {:?}", name, err));

    //when
    let result = fluvio_jolt::transform(input, &spec).unwrap();

    //then
    assert_eq!(result, expected, "failed assertion for test `{}`", name);
}
