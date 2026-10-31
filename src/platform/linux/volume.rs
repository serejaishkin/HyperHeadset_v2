use crate::system_audio;
use std::process::Command;

pub struct LinuxVolume;

fn mic_muted() -> Option<bool> {
    let out = Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"]).output().ok()?;
    if out.status.success() {
        return Some(String::from_utf8_lossy(&out.stdout).contains("[MUTED]"));
    }
    let out = Command::new("pactl").args(["get-source-mute", "@DEFAULT_SOURCE@"]).output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).contains("yes"))
}

impl LinuxVolume {
    pub fn new() -> Self { Self }

    pub fn get_master_volume(&self) -> Option<f32> {
        system_audio::get_levels().ok().map(|l| l.output as f32)
    }

    pub fn set_master_volume(&self, vol: f32) {
        let _ = system_audio::set_output(vol.clamp(0.0, 100.0).round() as u8);
    }

    pub fn get_microphone_volume(&self) -> Option<f32> {
        system_audio::get_levels().ok().map(|l| l.input as f32)
    }

    pub fn set_microphone_volume(&self, vol: f32) {
        let _ = system_audio::set_input(vol.clamp(0.0, 100.0).round() as u8);
    }

    pub fn get_microphone_mute(&self) -> Option<bool> {
        mic_muted()
    }

    pub fn set_microphone_mute(&self, mute: bool) {
        if mic_muted() != Some(mute) {
            let _ = system_audio::toggle_mic_mute();
        }
    }
}
