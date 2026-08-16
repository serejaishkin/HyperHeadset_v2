use std::collections::HashMap;
use std::path::Path;

pub struct I18n {
    strings: HashMap<String, String>,
}

impl I18n {
    pub fn new<P: AsRef<Path>>(_path: P, _lang: &str) -> Self {
        Self { strings: HashMap::new() }
    }

    pub fn t(&self, key: &str) -> String {
        self.strings.get(key).cloned().unwrap_or_else(|| key.to_string())
    }
}