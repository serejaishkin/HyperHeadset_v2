//! Linux Equalizer integration via EasyEffects / PipeWire
//!
//! Uses D-Bus to communicate with EasyEffects service.
//! Alternative: uses `easyeffects` CLI for preset management.

use std::process::Command;
use std::path::PathBuf;
use std::fs;

const EASYFX_PRESETS_DIR: &str = "easyeffects";
const PIPEWIRE_EQ_PLUGIN: &str = "equalizer";

/// Check if EasyEffects is installed and running
pub fn is_easyeffects_available() -> bool {
    Command::new("easyeffects")
        .arg("--help")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if PipeWire is running
pub fn is_pipewire_running() -> bool {
    Command::new("pgrep")
        .arg("pipewire")
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Apply EQ bands via EasyEffects D-Bus
/// 
/// EasyEffects exposes D-Bus interface:
///   bus name: com.github.wwmm.easyeffects
///   path: /com/github/wwmm/easyeffects/streaminputs/equalizer
///   method: set_input_gain(left_gain, right_gain)
pub fn apply_eq_bands(bands: &[f32; 10]) -> anyhow::Result<()> {
    // Method 1: Use easyeffects CLI to load a generated preset
    let preset_name = "hyperx_current";
    save_preset(preset_name, bands)?;

    let output = Command::new("easyeffects")
        .args(&["--load-preset", preset_name])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("EasyEffects failed: {}", stderr));
    }

    log::info!("[LinuxEQ] Applied {} bands via EasyEffects", bands.len());
    Ok(())
}

/// Save preset to EasyEffects config directory
pub fn save_preset(name: &str, bands: &[f32; 10]) -> anyhow::Result<()> {
    let mut config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("No config directory"))?;
    config_dir.push(EASYFX_PRESETS_DIR);
    config_dir.push("input");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let path = config_dir.join(format!("{}.json", name));

    // EasyEffects preset format (JSON)
    let freqs = [32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];

    let mut bands_json = Vec::new();
    for (i, (freq, gain)) in freqs.iter().zip(bands.iter()).enumerate() {
        bands_json.push(serde_json::json!({
            "frequency": freq,
            "gain": gain,
            "q": 1.41,
            "type": "Bell",
            "mode": "RLC (BT)",
            "slope": "x1",
            "solo": false,
            "mute": false
        }));
    }

    let preset = serde_json::json!({
        "input": {
            "equalizer": {
                "input-gain": -6.0,
                "output-gain": 0.0,
                "mode": "IIR",
                "num-bands": 10,
                "split-channels": false,
                "left": {
                    "type": "Bell",
                    "mode": "RLC (BT)",
                    "slope": "x1",
                    "solo": false,
                    "mute": false,
                    "gain": 0.0,
                    "frequency": 1000.0,
                    "q": 1.0
                },
                "right": {
                    "type": "Bell",
                    "mode": "RLC (BT)",
                    "slope": "x1",
                    "solo": false,
                    "mute": false,
                    "gain": 0.0,
                    "frequency": 1000.0,
                    "q": 1.0
                },
                "bands": bands_json
            }
        }
    });

    fs::write(&path, serde_json::to_string_pretty(&preset)?)?;
    log::info!("[LinuxEQ] Saved preset to {:?}", path);
    Ok(())
}

/// Load preset from EasyEffects config
pub fn load_preset(name: &str) -> anyhow::Result<[f32; 10]> {
    let mut config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("No config directory"))?;
    config_dir.push(EASYFX_PRESETS_DIR);
    config_dir.push("input");

    let path = config_dir.join(format!("{}.json", name));
    let content = fs::read_to_string(&path)?;
    let preset: serde_json::Value = serde_json::from_str(&content)?;

    let bands = preset
        .get("input")
        .and_then(|i| i.get("equalizer"))
        .and_then(|e| e.get("bands"))
        .and_then(|b| b.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid preset format"))?;

    let mut result = [0.0f32; 10];
    for (i, band) in bands.iter().take(10).enumerate() {
        if let Some(gain) = band.get("gain").and_then(|g| g.as_f64()) {
            result[i] = gain as f32;
        }
    }

    Ok(result)
}

/// List available presets
pub fn list_presets() -> Vec<String> {
    let mut config_dir = dirs::config_dir().unwrap_or_default();
    config_dir.push(EASYFX_PRESETS_DIR);
    config_dir.push("input");

    if !config_dir.exists() {
        return Vec::new();
    }

    let mut presets = Vec::new();
    if let Ok(entries) = fs::read_dir(&config_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".json") {
                presets.push(name.trim_end_matches(".json").to_string());
            }
        }
    }
    presets
}

/// Alternative: Use pw-cli to set PipeWire EQ directly (no EasyEffects needed)
/// This requires pipewire-filter-chain setup
pub fn apply_via_pipewire_cli(bands: &[f32; 10]) -> anyhow::Result<()> {
    let freqs = [32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];

    for (i, (freq, gain)) in freqs.iter().zip(bands.iter()).enumerate() {
        let _ = Command::new("pw-cli")
            .args(&[
                "s",
                "Filter-Chain",
                "Props",
                &format!("eq.{}.gain={}", i, gain),
                &format!("eq.{}.freq={}", i, freq),
            ])
            .output()?;
    }

    log::info!("[LinuxEQ] Applied bands via pw-cli");
    Ok(())
}
