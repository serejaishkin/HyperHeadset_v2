use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct I18n {
    strings: HashMap<String, String>,
    current_lang: String,
}

pub const DEFAULT_KEYS: &[&str] = &[
    "HyperX NGENUITY Open",
    "Dashboard",
    "Equalizer",
    "Settings",
    "Discord",
    "Headset",
    "Voice",
    "Tray Icon",
    "Debug",
    "Headset Settings",
    "Mute Button",
    "Voice Notifications",
    "Sidetone (hear yourself)",
    "Voice prompts",
    "Auto-shutdown:",
    "Apply",
    "Save",
    "Standard\n(MicMute)",
    "Play/Pause",
    "Smart: single=mute\ndouble=Play/Pause",
    "Smart: short=Play/Pause\nhold=mute",
    "Smart: short=mute\nhold=Play/Pause",
    "Always toggles microphone in Discord.",
    "Always pauses/plays media.",
    "Spotify, YouTube, VLC",
    "Single click (<400ms) → MicMute",
    "Double click (<400ms) → Play/Pause",
    "(!) 400ms delay",
    "Short press (<500ms) → Play/Pause",
    "Long hold (>500ms) → MicMute",
    "Short press (<500ms) → MicMute",
    "Long hold (>500ms) → Play/Pause",
    "(!) Requires down/up HID events",
    "Test: emulate press",
    "Test not available for hold modes",
    "Enable voice",
    "Battery low",
    "Charging",
    "Full charge",
    "Connected",
    "Disconnected",
    "Button check",
    "Exact percent",
    "Apply Voice Settings",
    "Tray Icon Settings",
    "Size:",
    "Font:",
    "Outline:",
    "Border:",
    "Gap:",
    "Colors",
    "Charging",
    "High (>50%)",
    "Medium (20-50%)",
    "Low (<20%)",
    "BG",
    "FG",
    "Out",
    "Bdr",
    "Preview",
    "Save Tray Icon Config",
    "Reset to Default",
    "Apply to Tray Now",
    "Debug level (verbose)",
    "Log to console",
    "Log to file (hyperx-ngenuity-open.log)",
    "Logging changes require restart.",
    "Battery",
    "Microphone",
    "Signal:",
    "dBm",
    "Quick Actions",
    "Mute Mic",
    "Unmute Mic",
    "Check Battery (Voice)",
    "MUTE",
    "MIC ON",
    "VOL",
    "MIC",
    "ON",
    "OFF - No connection",
    "Volume",
    "Mic Volume",
    "Headset Status",
    "Open",
    "Toggle Mute",
    "Quit",
    "⚡ Заряд",
    "🔋 Батарея",
    "🔇 Микрофон: выкл",
    "🎙️ Микрофон: вкл",
    "Открыть",
    "Переключить мьют",
    "Выход",
    "⚡ Заряжается",
    "🔋 Батарея",
    "🎤 Микрофон",
    "🔇 Выключен",
    "🎙️ Включён",
    "Equalizer",
    "Presets:",
    "Flat",
    "Bass Boost",
    "Bass Cut",
    "Treble Boost",
    "Voice Chat",
    "Gaming",
    "System Equalizer",
    "Enable system EQ",
    "[OK] Equalizer APO detected",
    "[ERR] Equalizer APO not found",
    "[OK] eqMac detected",
    "[ERR] eqMac not running",
    "EQ is applied at OS level",
    "Unsaved changes",
    "(!) Unsaved changes",
    "Sidetone",
    "PLAY",
];

impl I18n {
    pub fn new(lang_dir: &Path, lang: &str) -> Self {
        let mut strings = HashMap::new();
        let default_path = lang_dir.join("default.lang");
        if let Ok(content) = std::fs::read_to_string(&default_path) {
            Self::parse(&content, &mut strings);
        }
        if lang != "default" {
            let path = lang_dir.join(format!("{}.lang", lang));
            if let Ok(content) = std::fs::read_to_string(&path) {
                Self::parse(&content, &mut strings);
            }
        }
        Self { strings, current_lang: lang.to_string() }
    }

    fn parse(content: &str, map: &mut HashMap<String, String>) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                if !key.is_empty() {
                    map.insert(key, value);
                }
            }
        }
    }

    pub fn t(&self, key: &str) -> String {
        self.strings.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    pub fn current_lang(&self) -> &str { &self.current_lang }

    pub fn list_available(lang_dir: &Path) -> Vec<(String, String)> {
        let mut langs = vec![("default".to_string(), "English".to_string())];
        if let Ok(entries) = std::fs::read_dir(lang_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".lang") && name != "default.lang" {
                    let code = name.trim_end_matches(".lang").to_string();
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        for line in content.lines() {
                            if let Some(val) = line.strip_prefix("meta.name=") {
                                langs.push((code.clone(), val.trim().to_string()));
                                break;
                            }
                        }
                    }
                }
            }
        }
        langs
    }

    pub fn generate_default<P: AsRef<Path>>(lang_dir: P, keys: &[&str]) -> anyhow::Result<()> {
        std::fs::create_dir_all(&lang_dir)?;
        let mut content = String::from("# HyperX NGENUITY Open — Default Language File\n");
        content.push_str("# This file is auto-generated. Do not edit keys (left side of =).\n");
        content.push_str("# Copy this file, rename to <language>.lang, translate values.\n\n");
        content.push_str("meta.name=English\n\n");
        for key in keys {
            content.push_str(&format!("{}={}\n", key, key));
        }
        std::fs::write(lang_dir.as_ref().join("default.lang"), content)?;
        Ok(())
    }
}
