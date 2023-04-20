use std::fs;
use std::path::PathBuf;
use fluvio_jolt::{TransformSpec, transform};
use serde::Deserialize;
use serde_json::Value as JsonValue;

fn iter_json(dir_path: &str) -> Box<dyn Iterator<Item = (PathBuf, TestCase)>> {
    let dir = fs::read_dir(dir_path).unwrap();

    let iter = dir.into_iter().map(|entry| {
        let path = entry.unwrap().path();
        let contents = fs::read_to_string(&path).unwrap();
        let contents = contents
            .split('\n')
            .map(|line| {
                if let Some(idx) = line.find("//") {
                    &line[..idx]
                } else {
                    line
                }
            })
            .collect::<String>();
        let json = serde_json::from_str::<TestCase>(&contents).unwrap();

        (path, json)
    });

    Box::new(iter)
}

pub fn test_dir(dir_path: &str, operation: &str) {
    for (path, case) in iter_json(dir_path) {
        let val = serde_json::json!([{
            "operation": operation,
            "spec": case.spec,
        }]);
        let spec: TransformSpec = serde_json::from_value(val).unwrap();

        let output = transform(case.input, &spec);

        if output != case.expected {
            let expected = serde_json::to_string_pretty(&case.expected).unwrap();
            let output = serde_json::to_string_pretty(&output).unwrap();
            let path = path.to_str().unwrap();
            panic!("failed test;operation={operation};path={path};\nexpected={expected}\noutput={output}");
        }
    }
}

#[derive(Deserialize)]
struct TestCase {
    input: JsonValue,
    spec: JsonValue,
    expected: JsonValue,
}
