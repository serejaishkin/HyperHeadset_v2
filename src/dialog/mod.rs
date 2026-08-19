//! Cross-platform file dialogs for preset import/export
//!
//! Uses native dialogs via rfd crate (optional) or simple text input fallback.
//! Since we want zero extra dependencies, we use platform-specific approaches:
//! - Windows: COM IFileDialog
//! - Linux: zenity / kdialog fallback
//! - macOS: AppleScript osascript

use std::path::PathBuf;
use std::process::Command;

/// Open file dialog to select a preset file to import
pub fn open_import_dialog() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        open_file_dialog_windows("Import Preset", "HyperX Preset\0*.hyperx;*.txt;*.json\0")
    }

    #[cfg(target_os = "linux")]
    {
        open_file_dialog_linux("Import Preset", "~/")
    }

    #[cfg(target_os = "macos")]
    {
        open_file_dialog_macos("Import Preset")
    }
}

/// Save file dialog to export a preset
pub fn open_export_dialog(default_name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        save_file_dialog_windows("Export Preset", default_name, "HyperX Preset\0*.hyperx\0")
    }

    #[cfg(target_os = "linux")]
    {
        save_file_dialog_linux("Export Preset", default_name, "~/")
    }

    #[cfg(target_os = "macos")]
    {
        save_file_dialog_macos("Export Preset", default_name)
    }
}

// ===== Windows: COM IFileDialog =====
#[cfg(target_os = "windows")]
fn open_file_dialog_windows(title: &str, _filter: &str) -> Option<PathBuf> {
    // For now, use a simple approach with PowerShell
    let script = format!(
        r#"Add-Type -AssemblyName System.Windows.Forms; 
        $dlg = New-Object System.Windows.Forms.OpenFileDialog; 
        $dlg.Title = '{}'; 
        $dlg.Filter = 'HyperX Presets|*.hyperx;*.txt;*.json|All Files|*.*'; 
        $dlg.ShowDialog() | Out-Null; 
        if ($dlg.FileName) {{ $dlg.FileName }}"#,
        title
    );

    if let Ok(output) = Command::new("powershell")
        .args(&["-Command", &script])
        .output() 
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() && PathBuf::from(&path).exists() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn save_file_dialog_windows(title: &str, default_name: &str, _filter: &str) -> Option<PathBuf> {
    let script = format!(
        r#"Add-Type -AssemblyName System.Windows.Forms; 
        $dlg = New-Object System.Windows.Forms.SaveFileDialog; 
        $dlg.Title = '{}'; 
        $dlg.FileName = '{}'; 
        $dlg.Filter = 'HyperX Preset|*.hyperx|All Files|*.*'; 
        $dlg.ShowDialog() | Out-Null; 
        if ($dlg.FileName) {{ $dlg.FileName }}"#,
        title, default_name
    );

    if let Ok(output) = Command::new("powershell")
        .args(&["-Command", &script])
        .output() 
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

// ===== Linux: zenity / kdialog =====
#[cfg(target_os = "linux")]
fn open_file_dialog_linux(title: &str, _default_dir: &str) -> Option<PathBuf> {
    // Try zenity first
    if let Ok(output) = Command::new("zenity")
        .args(&["--file-selection", "--title", title, "--file-filter=HyperX Presets | *.hyperx *.txt *.json"])
        .output() 
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    // Fallback to kdialog
    if let Ok(output) = Command::new("kdialog")
        .args(&["--getopenfilename", ".", "HyperX Presets (*.hyperx *.txt *.json)"])
        .output() 
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn save_file_dialog_linux(title: &str, default_name: &str, _default_dir: &str) -> Option<PathBuf> {
    if let Ok(output) = Command::new("zenity")
        .args(&["--file-selection", "--save", "--title", title, "--filename", default_name])
        .output() 
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    if let Ok(output) = Command::new("kdialog")
        .args(&["--getsavefilename", default_name, "HyperX Presets (*.hyperx)"])
        .output() 
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    None
}

// ===== macOS: osascript =====
#[cfg(target_os = "macos")]
fn open_file_dialog_macos(title: &str) -> Option<PathBuf> {
    let script = format!(
        r#"osascript -e 'POSIX path of (choose file with prompt "{}")'"#,
        title
    );

    if let Ok(output) = Command::new("sh")
        .args(&["-c", &script])
        .output() 
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn save_file_dialog_macos(title: &str, default_name: &str) -> Option<PathBuf> {
    let script = format!(
        r#"osascript -e 'POSIX path of (choose file name with prompt "{}" default name "{}")'"#,
        title, default_name
    );

    if let Ok(output) = Command::new("sh")
        .args(&["-c", &script])
        .output() 
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

/// Preset file format: JSON with HyperX-specific structure
#[derive(serde::Serialize, serde::Deserialize)]
pub struct PresetFile {
    pub name: String,
    pub description: Option<String>,
    pub bands: [f32; 10],
    pub frequencies: [f32; 10],
    pub version: String,
}

impl PresetFile {
    pub fn new(name: &str, bands: [f32; 10]) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            bands,
            frequencies: [32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0],
            version: "1.0".to_string(),
        }
    }

    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let preset: PresetFile = serde_json::from_str(&content)?;
        Ok(preset)
    }
}
