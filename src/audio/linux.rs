//! Linux Equalizer (EasyEffects / PipeWire integration)
use std::path::PathBuf;
use std::fs;

const BAND_FREQUENCIES: [f32; 10] = [
    32.0, 64.0, 125.0, 250.0, 500.0,
    1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub fn apply_eq_bands(bands: &[f32; 10]) -> anyhow::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/root"));
    let preset_dir = PathBuf::from(home).join(".config/easyeffects/equalizer");
    if !preset_dir.exists() {
        fs::create_dir_all(&preset_dir)?;
    }
    let preset_path = preset_dir.join("hyperx_ngenuity.json");
    
    let mut bands_json = String::new();
    for (i, (freq, gain)) in BAND_FREQUENCIES.iter().zip(bands.iter()).enumerate() {
        bands_json.push_str(&format!(
            "    \"band {}\": {{\n      \"enabled\": true,\n      \"frequency\": {:.1},\n      \"gain\": {:.1},\n      \"q\": 1.41\n    }}{}\n",
            i, freq, gain, if i == 9 { "" } else { "," }
        ));
    }

    let json_content = format!(
        "{{\n  \"input-gain\": 0.0,\n  \"output-gain\": 0.0,\n  \"bands\": {{\n{0}}}\n}}\n",
        bands_json
    );

    fs::write(&preset_path, json_content)?;
    log::info!("[LinuxEQ] Written EasyEffects preset to {:?}", preset_path);
    
    let _ = std::process::Command::new("easyeffects")
        .arg("--load-preset")
        .arg("hyperx_ngenuity")
        .spawn();

    Ok(())
}
