//! macOS Equalizer (eqMac integration)
use std::path::PathBuf;
use std::fs;

const BAND_FREQUENCIES: [f32; 10] = [
    32.0, 64.0, 125.0, 250.0, 500.0,
    1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub fn apply_eq_bands(bands: &[f32; 10]) -> anyhow::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/Users/shared"));
    let preset_dir = PathBuf::from(home).join("Library/Application Support/com.bitgapp.eqMac/Presets");
    if !preset_dir.exists() {
        fs::create_dir_all(&preset_dir)?;
    }
    let preset_path = preset_dir.join("HyperXNGenuity.json");

    let mut bands_arr = String::new();
    for (i, (freq, gain)) in BAND_FREQUENCIES.iter().zip(bands.iter()).enumerate() {
        bands_arr.push_str(&format!(
            "    {{\n      \"index\": {},\n      \"frequency\": {:.1},\n      \"gain\": {:.1}\n    }}{}\n",
            i, freq, gain, if i == 9 { "" } else { "," }
        ));
    }

    let json_content = format!(
        "{{\n  \"name\": \"HyperX NGENUITY\",\n  \"gains\": [\n{0}]\n}}\n",
        bands_arr
    );

    fs::write(&preset_path, json_content)?;
    log::info!("[MacEQ] Written eqMac preset to {:?}", preset_path);
    Ok(())
}
