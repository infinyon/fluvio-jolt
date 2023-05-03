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
                let mut in_str = false;
                let mut maybe_comment = false;
                for (idx, c) in line.char_indices() {
                    match c {
                        '"' => {
                            in_str = !in_str;
                            maybe_comment = false;
                        }
                        '/' => {
                            if maybe_comment {
                                return &line[..idx - 1];
                            }
                            maybe_comment = !in_str;
                        }
                        _ => maybe_comment = false,
                    }
                }

                line
            })
            .collect::<Vec<_>>()
            .join("");
        let json = match serde_json::from_str::<TestCase>(&contents) {
            Ok(json) => json,
            Err(e) => {
                let path = path.to_str().unwrap();
                panic!("failed to deserialize test case at {path}:\n{e}\ninput was:\n{contents}");
            }
        };

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

        let spec: TransformSpec = match serde_json::from_value(val) {
            Ok(json) => json,
            Err(e) => {
                let path = path.to_str().unwrap();
                panic!("failed to deserialize test case at {path}.\n{e}");
            }
        };

        let output = transform(case.input, &spec).unwrap();

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
