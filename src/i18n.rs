use std::collections::HashMap;
use std::path::Path;

pub const DEFAULT_KEYS: &[&str] = &[];

pub struct I18n {
    strings: HashMap<String, String>,
    lang: String,
}

impl I18n {
    pub fn new<P: AsRef<Path>>(_path: P, lang: &str) -> Self {
        Self { strings: HashMap::new(), lang: lang.to_string() }
    }

    pub fn t(&self, key: &str) -> String {
        self.strings.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    pub fn current_lang(&self) -> &str {
        &self.lang
    }

    pub fn list_available<P: AsRef<Path>>(_path: P) -> Vec<(String, String)> {
        vec![("en".to_string(), "English".to_string()), ("ru".to_string(), "Русский".to_string())]
    }

    pub fn generate_default<P: AsRef<Path>>(_path: P, _keys: &[&str]) -> anyhow::Result<()> {
        Ok(())
    }
}
