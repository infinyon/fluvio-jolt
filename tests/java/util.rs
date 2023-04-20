use std::fs;
use std::path::{PathBuf, Path};
use serde_json::Value as JsonValue;

pub fn iter_json(dir_path: &Path) -> Box<dyn Iterator<Item = (PathBuf, JsonValue)>> {
    let dir = fs::read_dir(dir_path).unwrap();

    let iter = dir.into_iter().map(|entry| {
        let path = entry.unwrap().path();
        let contents = fs::read_to_string(&path).unwrap();
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
        let json = serde_json::from_str::<JsonValue>(&contents).unwrap();

        (path.to_owned(), json)
    });

    Box::new(iter)
}
