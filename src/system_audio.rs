//! Cross-platform system audio controls used by the Tauri frontend.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AudioLevels {
    pub output: u8,
    pub input: u8,
}

pub fn get_levels() -> anyhow::Result<AudioLevels> { platform::get_levels() }
pub fn set_output(percent: u8) -> anyhow::Result<()> { platform::set_output(percent.min(100)) }
pub fn set_input(percent: u8) -> anyhow::Result<()> { platform::set_input(percent.min(100)) }
pub fn toggle_mic_mute() -> anyhow::Result<()> { platform::toggle_mic_mute() }

pub fn play_pause() -> anyhow::Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    enigo.key(Key::MediaPlayPause, Direction::Click).map_err(|e| anyhow::anyhow!(e.to_string()))
}

#[cfg(target_os = "windows")]
mod platform {
    use super::AudioLevels;
    use windows::Win32::Media::Audio::{eCapture, eConsole, eRender, IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

    fn endpoint(flow: windows::Win32::Media::Audio::EDataFlow) -> anyhow::Result<IAudioEndpointVolume> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(flow, eConsole)?;
            Ok(device.Activate(CLSCTX_ALL, None)?)
        }
    }
    fn read(flow: windows::Win32::Media::Audio::EDataFlow) -> anyhow::Result<u8> {
        unsafe { Ok((endpoint(flow)?.GetMasterVolumeLevelScalar()? * 100.0).round().clamp(0.0, 100.0) as u8) }
    }
    fn set(flow: windows::Win32::Media::Audio::EDataFlow, percent: u8) -> anyhow::Result<()> {
        unsafe { endpoint(flow)?.SetMasterVolumeLevelScalar(percent as f32 / 100.0, std::ptr::null())?; }
        Ok(())
    }
    pub fn get_levels() -> anyhow::Result<AudioLevels> { Ok(AudioLevels { output: read(eRender)?, input: read(eCapture)? }) }
    pub fn set_output(percent: u8) -> anyhow::Result<()> { set(eRender, percent) }
    pub fn set_input(percent: u8) -> anyhow::Result<()> { set(eCapture, percent) }
    pub fn toggle_mic_mute() -> anyhow::Result<()> {
        unsafe {
            let ep = endpoint(eCapture)?;
            let muted = ep.GetMute()?.as_bool();
            ep.SetMute(!muted, std::ptr::null())?;
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::AudioLevels;
    use std::process::Command;
    fn run(args: &[&str]) -> anyhow::Result<String> {
        let out = Command::new(args[0]).args(&args[1..]).output()?;
        if !out.status.success() { return Err(anyhow::anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string())); }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
    fn parse_percent(s: &str) -> anyhow::Result<u8> {
        let token = s.split_whitespace().find(|t| t.ends_with('%')).ok_or_else(|| anyhow::anyhow!("Audio percentage not found"))?;
        Ok(token.trim_end_matches('%').parse::<u8>()?.min(100))
    }
    pub fn get_levels() -> anyhow::Result<AudioLevels> {
        let sink = run(&["pactl", "get-sink-volume", "@DEFAULT_SINK@"]) ?;
        let source = run(&["pactl", "get-source-volume", "@DEFAULT_SOURCE@"]) ?;
        Ok(AudioLevels { output: parse_percent(&sink)?, input: parse_percent(&source)? })
    }
    pub fn set_output(percent: u8) -> anyhow::Result<()> { run(&["pactl", "set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", percent)])?; Ok(()) }
    pub fn set_input(percent: u8) -> anyhow::Result<()> { run(&["pactl", "set-source-volume", "@DEFAULT_SOURCE@", &format!("{}%", percent)])?; Ok(()) }
    pub fn toggle_mic_mute() -> anyhow::Result<()> { run(&["pactl", "set-source-mute", "@DEFAULT_SOURCE@", "toggle"])?; Ok(()) }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::AudioLevels;
    use std::process::Command;
    fn osa(script: &str) -> anyhow::Result<String> {
        let out = Command::new("osascript").args(["-e", script]).output()?;
        if !out.status.success() { return Err(anyhow::anyhow!(String::from_utf8_lossy(&out.stderr).trim().to_string())); }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
    fn parse(s: String) -> anyhow::Result<u8> { Ok(s.parse::<u8>()?.min(100)) }
    pub fn get_levels() -> anyhow::Result<AudioLevels> { Ok(AudioLevels { output: parse(osa("output volume of (get volume settings)")?)?, input: parse(osa("input volume of (get volume settings)")?)? }) }
    pub fn set_output(percent: u8) -> anyhow::Result<()> { osa(&format!("set volume output volume {}", percent))?; Ok(()) }
    pub fn set_input(percent: u8) -> anyhow::Result<()> { osa(&format!("set volume input volume {}", percent))?; Ok(()) }
    pub fn toggle_mic_mute() -> anyhow::Result<()> { Err(anyhow::anyhow!("macOS does not expose default input mute through osascript")) }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod platform {
    use super::AudioLevels;
    pub fn get_levels() -> anyhow::Result<AudioLevels> { Err(anyhow::anyhow!("System audio controls are not implemented for this platform")) }
    pub fn set_output(_: u8) -> anyhow::Result<()> { Err(anyhow::anyhow!("System audio controls are not implemented for this platform")) }
    pub fn set_input(_: u8) -> anyhow::Result<()> { Err(anyhow::anyhow!("System audio controls are not implemented for this platform")) }
    pub fn toggle_mic_mute() -> anyhow::Result<()> { Err(anyhow::anyhow!("System microphone controls are not implemented for this platform")) }
}
