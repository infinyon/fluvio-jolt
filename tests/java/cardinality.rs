use super::util::iter_json;
use std::path::Path;
use serde_json::Value as JsonValue;

#[test]
fn test_cardinality_transform() {
    for (path, json) in iter_json(Path::new("tests/java/resources/cardinality")) {
        todo!()
    }
}
