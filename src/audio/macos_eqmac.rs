//! macOS eqMac integration via HTTP API
//!
//! eqMac exposes a local HTTP API on localhost:8080
//! Documentation: https://github.com/bitgapp/eqMac/blob/master/docs/HTTP_API.md

use std::process::Command;

const EQMAC_API: &str = "http://localhost:8080/api";

/// Check if eqMac is running
pub fn is_eqmac_running() -> bool {
    if let Ok(output) = Command::new("pgrep").arg("eqMac").output() {
        !output.stdout.is_empty()
    } else {
        false
    }
}

/// Apply EQ bands to eqMac
pub async fn apply_eq_bands(bands: &[f32; 10]) -> anyhow::Result<()> {
    let freqs = [32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];

    let gains: Vec<f64> = bands.iter().map(|g| *g as f64).collect();

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/eq", EQMAC_API))
        .json(&serde_json::json!({
            "gains": gains,
            "frequencies": freqs,
        }))
        .send()
        .await?;

    if response.status().is_success() {
        log::info!("[eqMac] EQ applied successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!("eqMac API error: {}", response.status()))
    }
}

/// Save preset to eqMac
pub async fn save_preset(name: &str, bands: &[f32; 10]) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/presets", EQMAC_API))
        .json(&serde_json::json!({
            "name": name,
            "gains": bands,
        }))
        .send()
        .await?;

    if response.status().is_success() {
        log::info!("[eqMac] Preset '{}' saved", name);
        Ok(())
    } else {
        Err(anyhow::anyhow!("eqMac API error: {}", response.status()))
    }
}

/// Load preset from eqMac
pub async fn load_preset(name: &str) -> anyhow::Result<[f32; 10]> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/presets/{}", EQMAC_API, name))
        .send()
        .await?;

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        if let Some(gains) = data.get("gains").and_then(|g| g.as_array()) {
            let mut bands = [0.0f32; 10];
            for (i, gain) in gains.iter().take(10).enumerate() {
                bands[i] = gain.as_f64().unwrap_or(0.0) as f32;
            }
            return Ok(bands);
        }
    }

    Err(anyhow::anyhow!("Failed to load preset '{}'", name))
}

/// List eqMac presets
pub async fn list_presets() -> anyhow::Result<Vec<String>> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/presets", EQMAC_API))
        .send()
        .await?;

    if response.status().is_success() {
        let data: Vec<serde_json::Value> = response.json().await?;
        let names: Vec<String> = data
            .iter()
            .filter_map(|p| p.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
            .collect();
        Ok(names)
    } else {
        Ok(Vec::new())
    }
}
