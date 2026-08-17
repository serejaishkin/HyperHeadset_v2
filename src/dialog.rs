use std::path::PathBuf;

pub fn open_import_dialog() -> Option<PathBuf> {
    None
}

pub fn open_export_dialog(_default_name: &str) -> Option<PathBuf> {
    None
}

#[derive(Debug, Clone)]
pub struct PresetFile {
    pub name: String,
    pub bands: [f32; 10],
}

impl PresetFile {
    pub fn new(name: &str, bands: [f32; 10]) -> Self {
        Self { name: name.to_string(), bands }
    }

    pub fn load(_path: &PathBuf) -> anyhow::Result<Self> {
        anyhow::bail!("Preset import not implemented")
    }

    pub fn save(&self, _path: &PathBuf) -> anyhow::Result<()> {
        Ok(())
    }
}
