use std::str::FromStr;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct JsonPointer {
    entries: Vec<String>,
}

impl JsonPointer {
    pub(crate) fn new(mut entries: Vec<String>) -> Self {
        if entries.get(0).filter(|p| (**p).eq("")).is_none() {
            entries.insert(0, String::new());
        }
        Self { entries }
    }

    pub(crate) fn from_dot_notation(path: &str) -> Self {
        Self::new(path.split('.').map(|s| s.to_string()).collect())
    }

    pub(crate) fn push<T: ToString>(&mut self, value: T) {
        self.entries.push(value.to_string());
    }

    pub(crate) fn parent(&self) -> Self {
        let mut entries = self.entries.clone();
        entries.pop();
        Self::new(entries)
    }

    pub(crate) fn entries(&self) -> &[String] {
        &self.entries
    }

    pub(crate) fn leaf_name(&self) -> &str {
        self.entries.last().map(|s| s.as_str()).unwrap_or("")
    }
    /// Returns path elements of the pointer. First element is always empty string that corresponds
    /// to root level.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &String> {
        self.entries.iter()
    }

    /// Represents the pointer as [String] with the format [RFC6901](https://datatracker.ietf.org/doc/html/rfc6901).
    pub(crate) fn join_rfc6901(&self) -> String {
        self.entries.join("/")
    }

    /// Finds all path elements with the format '&N' and replaces them by values from  
    /// given bindings where N is the index of bindings slice.
    pub(crate) fn substitute_vars<T: ToString>(&mut self, bindings: &[T]) {
        for entry in self.entries.iter_mut() {
            if entry.starts_with('&') {
                if let Ok(index) = usize::from_str(&entry[1..entry.len()]) {
                    if let Some(var_value) = bindings.get(index) {
                        *entry = var_value.to_string();
                    }
                }
            }
        }
    }
}

impl Default for JsonPointer {
    fn default() -> Self {
        Self::new(Vec::default())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_root_level() {
        //given

        //when
        let pointer1 = JsonPointer::from_dot_notation("a.b");
        let pointer2 = JsonPointer::from_dot_notation(".a.b");
        let pointer3 = JsonPointer::new(vec!["".to_string(), "a".to_string()]);
        let pointer4 = JsonPointer::new(vec!["c".to_string(), "a".to_string()]);

        //then
        assert_eq!(pointer1.entries()[0], "");
        assert_eq!(pointer2.entries()[0], "");
        assert_eq!(pointer3.entries()[0], "");
        assert_eq!(pointer4.entries()[0], "");
    }

    #[test]
    fn test_parent() {
        //given
        let pointer = JsonPointer::from_dot_notation("a.b.c.d");

        //when
        let parent1 = pointer.parent();
        let parent2 = parent1.parent();
        let parent3 = parent2.parent();
        let parent4 = parent3.parent();
        let parent5 = parent4.parent();

        //then
        assert_eq!(parent1.join_rfc6901(), "/a/b/c");
        assert_eq!(parent2.join_rfc6901(), "/a/b");
        assert_eq!(parent3.join_rfc6901(), "/a");
        assert_eq!(parent4.join_rfc6901(), "");
        assert_eq!(parent5.join_rfc6901(), "");
    }

    #[test]
    fn test_substitute() {
        //given
        let mut pointer = JsonPointer::from_dot_notation(".&2.&1.&0");
        let bindings = &["d", "e", "g"];

        //when
        pointer.substitute_vars(bindings);

        //then
        assert_eq!(pointer.join_rfc6901(), "/g/e/d")
    }

    #[test]
    fn test_substitute_no_vars() {
        //given
        let mut pointer = JsonPointer::from_dot_notation(".a.b.c");
        let bindings = &["d", "e", "g"];

        //when
        pointer.substitute_vars(bindings);

        //then
        assert_eq!(pointer.join_rfc6901(), "/a/b/c")
    }

    #[test]
    fn test_substitute_with_empty_bindings() {
        //given
        let mut pointer = JsonPointer::from_dot_notation(".a.b.&0");
        let bindings: &[&str] = &[];

        //when
        pointer.substitute_vars(bindings);

        //then
        assert_eq!(pointer.join_rfc6901(), "/a/b/&0")
    }

    #[test]
    fn test_substitute_with_out_of_range_index() {
        //given
        let mut pointer = JsonPointer::from_dot_notation(".a.b.&11");
        let bindings = &["d", "e", "g"];

        //when
        pointer.substitute_vars(bindings);

        //then
        assert_eq!(pointer.join_rfc6901(), "/a/b/&11")
    }
}
