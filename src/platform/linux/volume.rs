use std::process::Command;

pub struct LinuxVolume;

impl LinuxVolume {
    pub fn new() -> Self { Self }

    // ========== MASTER (sinks) ==========
    pub fn get_master_volume(&self) -> Option<f32> {
        let output = Command::new("pactl")
            .args(["list", "sinks"])
            .output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("Volume:") && line.contains("front-left") {
                if let Some(start) = line.find("/ ") {
                    if let Some(end) = line[start+2..].find("%") {
                        let vol_str = &line[start+2..start+2+end];
                        return vol_str.trim().parse::<f32>().ok();
                    }
                }
            }
        }
        None
    }

    pub fn set_master_volume(&self, percent: f32) -> bool {
        let vol = percent.round() as i32;
        Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", vol)])
            .status().map(|s| s.success()).unwrap_or(false)
    }

    pub fn get_mute(&self) -> Option<bool> {
        let output = Command::new("pactl")
            .args(["list", "sinks"])
            .output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("Mute:") {
                return Some(line.contains("yes"));
            }
        }
        None
    }

    pub fn set_mute(&self, muted: bool) -> bool {
        let arg = if muted { "1" } else { "0" };
        Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", arg])
            .status().map(|s| s.success()).unwrap_or(false)
    }

    // ========== MICROPHONE (sources) ==========
    pub fn get_microphone_volume(&self) -> Option<f32> {
        let output = Command::new("pactl")
            .args(["list", "sources"])
            .output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("Volume:") && line.contains("front-left") {
                if let Some(start) = line.find("/ ") {
                    if let Some(end) = line[start+2..].find("%") {
                        let vol_str = &line[start+2..start+2+end];
                        return vol_str.trim().parse::<f32>().ok();
                    }
                }
            }
        }
        None
    }

    pub fn set_microphone_volume(&self, percent: f32) -> bool {
        let vol = percent.round() as i32;
        Command::new("pactl")
            .args(["set-source-volume", "@DEFAULT_SOURCE@", &format!("{}%", vol)])
            .status().map(|s| s.success()).unwrap_or(false)
    }

    pub fn get_microphone_mute(&self) -> Option<bool> {
        let output = Command::new("pactl")
            .args(["list", "sources"])
            .output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("Mute:") {
                return Some(line.contains("yes"));
            }
        }
        None
    }

    pub fn set_microphone_mute(&self, muted: bool) -> bool {
        let arg = if muted { "1" } else { "0" };
        Command::new("pactl")
            .args(["set-source-mute", "@DEFAULT_SOURCE@", arg])
            .status().map(|s| s.success()).unwrap_or(false)
    }
}