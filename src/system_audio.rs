//! Cross-platform system audio controls used by the Tauri frontend.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AudioLevels { pub output: u8, pub input: u8 }
pub fn get_levels() -> anyhow::Result<AudioLevels> { platform::get_levels() }
pub fn set_output(percent: u8) -> anyhow::Result<()> { platform::set_output(percent.min(100)) }
pub fn set_input(percent: u8) -> anyhow::Result<()> { platform::set_input(percent.min(100)) }
pub fn toggle_mic_mute() -> anyhow::Result<()> { platform::toggle_mic_mute() }
pub fn toggle_output_mute() -> anyhow::Result<()> { platform::toggle_output_mute() }
pub fn play_pause() -> anyhow::Result<()> { use enigo::{Direction, Enigo, Key, Keyboard, Settings}; let mut e=Enigo::new(&Settings::default()).map_err(|x| anyhow::anyhow!(x.to_string()))?; e.key(Key::MediaPlayPause,Direction::Click).map_err(|x| anyhow::anyhow!(x.to_string())) }

#[cfg(target_os="windows")]
mod platform {
 use super::AudioLevels;
 use windows::Win32::Media::Audio::{eCapture,eConsole,eRender,IMMDeviceEnumerator,MMDeviceEnumerator};
 use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
 use windows::Win32::System::Com::{CoCreateInstance,CoInitializeEx,CLSCTX_ALL,COINIT_MULTITHREADED};
 fn endpoint(flow:windows::Win32::Media::Audio::EDataFlow)->anyhow::Result<IAudioEndpointVolume>{unsafe{let _=CoInitializeEx(None,COINIT_MULTITHREADED);let en:IMMDeviceEnumerator=CoCreateInstance(&MMDeviceEnumerator,None,CLSCTX_ALL)?;let d=en.GetDefaultAudioEndpoint(flow,eConsole)?;Ok(d.Activate(CLSCTX_ALL,None)?)}}
 fn read(f:windows::Win32::Media::Audio::EDataFlow)->anyhow::Result<u8>{unsafe{Ok((endpoint(f)?.GetMasterVolumeLevelScalar()?*100.0).round().clamp(0.0,100.0)as u8)}}
 fn set(f:windows::Win32::Media::Audio::EDataFlow,p:u8)->anyhow::Result<()>{unsafe{endpoint(f)?.SetMasterVolumeLevelScalar(p as f32/100.0,std::ptr::null())?;}Ok(())}
 pub fn get_levels()->anyhow::Result<AudioLevels>{Ok(AudioLevels{output:read(eRender)?,input:read(eCapture)?})}
 pub fn set_output(p:u8)->anyhow::Result<()>{set(eRender,p)} pub fn set_input(p:u8)->anyhow::Result<()>{set(eCapture,p)}
 pub fn toggle_mic_mute()->anyhow::Result<()>{unsafe{let e=endpoint(eCapture)?;let m=e.GetMute()?.as_bool();e.SetMute(!m,std::ptr::null())?;}Ok(())}
 pub fn toggle_output_mute()->anyhow::Result<()>{unsafe{let e=endpoint(eRender)?;let m=e.GetMute()?.as_bool();e.SetMute(!m,std::ptr::null())?;}Ok(())}
}

#[cfg(target_os="linux")]
mod platform {
 use super::AudioLevels; use std::process::Command;
 fn run(p:&str,a:&[&str])->anyhow::Result<String>{let o=Command::new(p).args(a).output().map_err(|e|anyhow::anyhow!("{}: {}",p,e))?;if !o.status.success(){return Err(anyhow::anyhow!("{} failed: {}",p,String::from_utf8_lossy(&o.stderr).trim()))}Ok(String::from_utf8_lossy(&o.stdout).to_string())}
 fn avail(p:&str)->bool{Command::new(p).arg("--version").output().is_ok()} fn wp(a:&[&str])->anyhow::Result<String>{run("wpctl",a)} fn pa(a:&[&str])->anyhow::Result<String>{run("pactl",a)}
 fn vol(w:bool,s:bool)->anyhow::Result<u8>{if w{let o=wp(&["get-volume",if s{"@DEFAULT_AUDIO_SINK@"}else{"@DEFAULT_AUDIO_SOURCE@"}])?;return Ok((o.split_whitespace().next().ok_or_else(||anyhow::anyhow!("wpctl volume not found"))?.parse::<f32>()?*100.0).round().clamp(0.0,100.0)as u8)}let o=if s{pa(&["get-sink-volume","@DEFAULT_SINK@"]) ?}else{pa(&["get-source-volume","@DEFAULT_SOURCE@"]) ?};let t=o.split_whitespace().find(|x|x.ends_with('%')).ok_or_else(||anyhow::anyhow!("volume not found"))?;Ok(t.trim_end_matches('%').parse::<u8>()?.min(100))}
 pub fn get_levels()->anyhow::Result<AudioLevels>{let w=avail("wpctl");Ok(AudioLevels{output:vol(w,true)?,input:vol(w,false)?})}
 pub fn set_output(p:u8)->anyhow::Result<()>{if avail("wpctl"){wp(&["set-volume","@DEFAULT_AUDIO_SINK@",&format!("{}%",p)])?}else{pa(&["set-sink-volume","@DEFAULT_SINK@",&format!("{}%",p)])?}Ok(())}
 pub fn set_input(p:u8)->anyhow::Result<()>{if avail("wpctl"){wp(&["set-volume","@DEFAULT_AUDIO_SOURCE@",&format!("{}%",p)])?}else{pa(&["set-source-volume","@DEFAULT_SOURCE@",&format!("{}%",p)])?}Ok(())}
 pub fn toggle_mic_mute()->anyhow::Result<()>{if avail("wpctl"){wp(&["set-mute","@DEFAULT_AUDIO_SOURCE@","toggle"])?}else{pa(&["set-source-mute","@DEFAULT_SOURCE@","toggle"])?}Ok(())}
 pub fn toggle_output_mute()->anyhow::Result<()>{if avail("wpctl"){wp(&["set-mute","@DEFAULT_AUDIO_SINK@","toggle"])?}else{pa(&["set-sink-mute","@DEFAULT_SINK@","toggle"])?}Ok(())}
}

#[cfg(target_os="macos")]
mod platform {
 use super::AudioLevels; use std::process::Command;
 mod coreaudio_mute { include!("system_audio_macos.rs"); }
 fn osa(s:&str)->anyhow::Result<String>{let o=Command::new("osascript").args(["-e",s]).output()?;if !o.status.success(){return Err(anyhow::anyhow!(String::from_utf8_lossy(&o.stderr).trim().to_string()))}Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())}
 fn parse(s:String)->anyhow::Result<u8>{Ok(s.parse::<u8>()?.min(100))}
 pub fn get_levels()->anyhow::Result<AudioLevels>{Ok(AudioLevels{output:parse(osa("output volume of (get volume settings)")?)?,input:parse(osa("input volume of (get volume settings)")?)?})}
 pub fn set_output(p:u8)->anyhow::Result<()>{osa(&format!("set volume output volume {}",p))?;Ok(())}
 pub fn set_input(p:u8)->anyhow::Result<()>{osa(&format!("set volume input volume {}",p))?;Ok(())}
 pub fn toggle_mic_mute()->anyhow::Result<()>{coreaudio_mute::toggle_input_mute()}
 pub fn toggle_output_mute()->anyhow::Result<()>{osa("set volume output muted not (output muted of (get volume settings))")?;Ok(())}
}

#[cfg(not(any(target_os="windows",target_os="linux",target_os="macos")))]
mod platform { use super::AudioLevels; pub fn get_levels()->anyhow::Result<AudioLevels>{Err(anyhow::anyhow!("System audio controls are not implemented for this platform"))} pub fn set_output(_:u8)->anyhow::Result<()>{Err(anyhow::anyhow!("System audio controls are not implemented for this platform"))} pub fn set_input(_:u8)->anyhow::Result<()>{Err(anyhow::anyhow!("System audio controls are not implemented for this platform"))} pub fn toggle_mic_mute()->anyhow::Result<()>{Err(anyhow::anyhow!("System microphone controls are not implemented for this platform"))} pub fn toggle_output_mute()->anyhow::Result<()>{Err(anyhow::anyhow!("System output mute is not implemented for this platform"))} }
