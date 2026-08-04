use std::process::Command;

pub struct MacOSVolume;

impl MacOSVolume {
    pub fn new() -> Self { Self }

    fn run_osascript(args: &[&str]) -> Option<String> {
        let output = Command::new("osascript")
            .args(args)
            .output().ok()?;
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn get_master_volume(&self) -> Option<f32> {
        let out = Self::run_osascript(&["-e", "get volume settings"])?;
        for part in out.split(',') {
            if part.contains("output volume") {
                let val = part.split(':').nth(1)?.trim();
                return val.parse::<f32>().ok();
            }
        }
        None
    }

    pub fn set_master_volume(&self, percent: f32) -> bool {
        let vol = percent.round() as i32;
        Command::new("osascript")
            .arg("-e")
            .arg(format!("set volume output volume {}", vol))
            .status().map(|s| s.success()).unwrap_or(false)
    }

    pub fn get_mute(&self) -> Option<bool> {
        let out = Self::run_osascript(&["-e", "get volume settings"])?;
        Some(out.contains("output muted: true"))
    }

    pub fn set_mute(&self, muted: bool) -> bool {
        let arg = if muted { "true" } else { "false" };
        Command::new("osascript")
            .arg("-e")
            .arg(format!("set volume with output muted {}", arg))
            .status().map(|s| s.success()).unwrap_or(false)
    }

    pub fn get_microphone_volume(&self) -> Option<f32> {
        let out = Self::run_osascript(&["-e", "get volume settings"])?;
        for part in out.split(',') {
            if part.contains("input volume") {
                let val = part.split(':').nth(1)?.trim();
                return val.parse::<f32>().ok();
            }
        }
        None
    }

    pub fn set_microphone_volume(&self, percent: f32) -> bool {
        let vol = percent.round() as i32;
        Command::new("osascript")
            .arg("-e")
            .arg(format!("set volume input volume {}", vol))
            .status().map(|s| s.success()).unwrap_or(false)
    }

    pub fn get_microphone_mute(&self) -> Option<bool> {
        self.get_microphone_volume().map(|v| v == 0.0)
    }

    pub fn set_microphone_mute(&self, muted: bool) -> bool {
        if muted {
            self.set_microphone_volume(0.0)
        } else {
            self.set_microphone_volume(50.0)
        }
    }
}
