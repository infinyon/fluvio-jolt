use std::fs;
use std::path::Path;
use serde_json::Value as JsonValue;

pub fn for_each_file<F: Fn(JsonValue) -> ()>(dir_path: &Path, f: F) {
    let dir = fs::read_dir(dir_path).unwrap();

    for entry in dir.into_iter() {
        let contents = fs::read_to_string(entry.unwrap().path()).unwrap();
        let contents = contents
            .split("\n")
            .map(|line| {
                if let Some(idx) = line.find("//") {
                    &line[..idx]
                } else {
                    line
                }
            })
            .collect::<String>();
        f(serde_json::from_str(&contents).unwrap());
    }
}
