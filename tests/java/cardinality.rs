use super::util::for_each_file;
use std::path::Path;
use serde_json::Value as JsonValue;

#[test]
fn test_cardinality_transform() {
    for_each_file(Path::new("tests/java/resources/cardinality"), test_case);
}

fn test_case(json: JsonValue) {
    todo!()
}
