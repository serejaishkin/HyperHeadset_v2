//! Cross-platform system audio controls used by the Tauri frontend.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AudioLevels { pub output: u8, pub input: u8 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_output: bool,
    pub is_input: bool,
}

pub fn get_levels() -> anyhow::Result<AudioLevels> { platform::get_levels() }
pub fn set_output(percent: u8) -> anyhow::Result<()> { platform::set_output(percent.min(100)) }
pub fn set_input(percent: u8) -> anyhow::Result<()> { platform::set_input(percent.min(100)) }
pub fn toggle_mic_mute() -> anyhow::Result<()> { platform::toggle_mic_mute() }
pub fn toggle_output_mute() -> anyhow::Result<()> { platform::toggle_output_mute() }
pub fn play_pause() -> anyhow::Result<()> { use enigo::{Direction, Enigo, Key, Keyboard, Settings}; let mut e=Enigo::new(&Settings::default()).map_err(|x| anyhow::anyhow!(x.to_string()))?; e.key(Key::MediaPlayPause,Direction::Click).map_err(|x| anyhow::anyhow!(x.to_string())) }
pub fn get_audio_devices() -> anyhow::Result<Vec<AudioDevice>> { platform::get_audio_devices() }
pub fn set_default_output_device(device_id: &str) -> anyhow::Result<()> { platform::set_default_output_device(device_id) }
pub fn set_default_input_device(device_id: &str) -> anyhow::Result<()> { platform::set_default_input_device(device_id) }

#[cfg(target_os="windows")]
mod platform {
 use super::{AudioLevels, AudioDevice};
 use windows::Win32::Media::Audio::{eCapture,eConsole,eRender,IMMDeviceEnumerator,MMDeviceEnumerator,DEVICE_STATE_ACTIVE};
 use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
 use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
 use windows::Win32::System::Com::StructuredStorage::PropVariantToStringWithDefault;
 use windows::Win32::System::Com::STGM;
 use windows::Win32::System::Com::{CoCreateInstance,CoInitializeEx,CLSCTX_ALL,COINIT_MULTITHREADED};
 fn endpoint(flow:windows::Win32::Media::Audio::EDataFlow)->anyhow::Result<IAudioEndpointVolume>{unsafe{let _=CoInitializeEx(None,COINIT_MULTITHREADED);let en:IMMDeviceEnumerator=CoCreateInstance(&MMDeviceEnumerator,None,CLSCTX_ALL)?;let d=en.GetDefaultAudioEndpoint(flow,eConsole)?;Ok(d.Activate(CLSCTX_ALL,None)?)}}
 fn read(f:windows::Win32::Media::Audio::EDataFlow)->anyhow::Result<u8>{unsafe{Ok((endpoint(f)?.GetMasterVolumeLevelScalar()?*100.0).round().clamp(0.0,100.0)as u8)}}
 fn set(f:windows::Win32::Media::Audio::EDataFlow,p:u8)->anyhow::Result<()>{unsafe{endpoint(f)?.SetMasterVolumeLevelScalar(p as f32/100.0,std::ptr::null())?;}Ok(())}
 pub fn get_levels()->anyhow::Result<AudioLevels>{Ok(AudioLevels{output:read(eRender)?,input:read(eCapture)?})}
 pub fn set_output(p:u8)->anyhow::Result<()>{set(eRender,p)} pub fn set_input(p:u8)->anyhow::Result<()>{set(eCapture,p)}
 pub fn toggle_mic_mute()->anyhow::Result<()>{unsafe{let e=endpoint(eCapture)?;let m=e.GetMute()?.as_bool();e.SetMute(!m,std::ptr::null())?;}Ok(())}
 pub fn toggle_output_mute()->anyhow::Result<()>{unsafe{let e=endpoint(eRender)?;let m=e.GetMute()?.as_bool();e.SetMute(!m,std::ptr::null())?;}Ok(())}

 pub fn get_audio_devices() -> anyhow::Result<Vec<AudioDevice>> {
   unsafe {
     let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
     let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
     let mut devices = Vec::new();

     // Get output devices
     let collection = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
     let count = collection.GetCount()?;
     for i in 0..count {
       if let Ok(device) = collection.Item(i) {
         if let Ok(id) = device.GetId() {
           let id = id.to_string()?;
           if let Ok(store) = device.OpenPropertyStore(STGM(0)) {
             if let Ok(value) = store.GetValue(&PKEY_Device_FriendlyName) {
               let name = PropVariantToStringWithDefault(&value as *const _, windows::core::PCWSTR::null())
                 .to_string()
                 .unwrap_or_else(|_| id.clone());
               devices.push(AudioDevice {
                 id,
                 name,
                 is_output: true,
                 is_input: false,
               });
             }
           }
         }
       }
     }

     // Get input devices
     let collection = en.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;
     let count = collection.GetCount()?;
     for i in 0..count {
       if let Ok(device) = collection.Item(i) {
         if let Ok(id) = device.GetId() {
           let id = id.to_string()?;
           if let Ok(store) = device.OpenPropertyStore(STGM(0)) {
             if let Ok(value) = store.GetValue(&PKEY_Device_FriendlyName) {
               let name = PropVariantToStringWithDefault(&value as *const _, windows::core::PCWSTR::null())
                 .to_string()
                 .unwrap_or_else(|_| id.clone());
               devices.push(AudioDevice {
                 id,
                 name,
                 is_output: false,
               is_input: true,
               });
             }
           }
         }
       }
     }

     Ok(devices)
   }
 }

 fn set_default_endpoint_native(device_id: &str) -> anyhow::Result<()> {
   use std::ffi::c_void;
   use windows::core::{GUID, IUnknown, PCWSTR};

   const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
   const ROLE_CONSOLE: i32 = 0;
   const ROLE_MULTIMEDIA: i32 = 1;
   const ROLE_COMMUNICATIONS: i32 = 2;

   #[repr(C)]
   struct PolicyConfigVTable {
     query_interface: *const c_void,
     add_ref: *const c_void,
     release: *const c_void,
     get_mix_format: *const c_void,
     get_device_format: *const c_void,
     reset_device_format: *const c_void,
     set_device_format: *const c_void,
     get_processing_period: *const c_void,
     set_processing_period: *const c_void,
     get_share_mode: *const c_void,
     set_share_mode: *const c_void,
     get_property_value: *const c_void,
     set_property_value: *const c_void,
     set_default_endpoint: unsafe extern "system" fn(*mut c_void, PCWSTR, i32) -> windows::core::HRESULT,
     set_endpoint_visibility: *const c_void,
   }

   unsafe {
     let policy: IUnknown = windows::Win32::System::Com::CoCreateInstance(
       &CLSID_POLICY_CONFIG_CLIENT,
       None,
       CLSCTX_ALL,
     )?;
     let raw = policy.as_raw();
     let vtbl = *(raw as *const *const PolicyConfigVTable);
     let wide: Vec<u16> = device_id.encode_utf16().chain(std::iter::once(0)).collect();
     let endpoint = PCWSTR(wide.as_ptr());

     for role in [ROLE_CONSOLE, ROLE_MULTIMEDIA, ROLE_COMMUNICATIONS] {
       let hr = (vtbl.set_default_endpoint)(raw as *mut c_void, endpoint, role);
       hr.ok().map_err(|e| anyhow::anyhow!("SetDefaultEndpoint failed for role {}: {}", role, e))?;
     }
   }
   Ok(())
 }

 pub fn set_default_output_device(device_id: &str) -> anyhow::Result<()> {
   set_default_endpoint_native(device_id)
 }

 pub fn set_default_input_device(device_id: &str) -> anyhow::Result<()> {
   set_default_endpoint_native(device_id)
 }
}

#[cfg(target_os="linux")]
mod platform {
 use super::{AudioLevels, AudioDevice}; use std::process::Command;
 fn run(p:&str,a:&[&str])->anyhow::Result<String>{let o=Command::new(p).args(a).output().map_err(|e|anyhow::anyhow!("{}: {}",p,e))?;if !o.status.success(){return Err(anyhow::anyhow!("{} failed: {}",p,String::from_utf8_lossy(&o.stderr).trim()))}Ok(String::from_utf8_lossy(&o.stdout).to_string())}
 fn avail(p:&str)->bool{Command::new(p).arg("--version").output().is_ok()} fn wp(a:&[&str])->anyhow::Result<String>{run("wpctl",a)} fn pa(a:&[&str])->anyhow::Result<String>{run("pactl",a)}
 fn vol(w:bool,s:bool)->anyhow::Result<u8>{if w{let o=wp(&["get-volume",if s{"@DEFAULT_AUDIO_SINK@"}else{"@DEFAULT_AUDIO_SOURCE@"}])?;return Ok((o.split_whitespace().next().ok_or_else(||anyhow::anyhow!("wpctl volume not found"))?.parse::<f32>()?*100.0).round().clamp(0.0,100.0)as u8)}let o=if s{pa(&["get-sink-volume","@DEFAULT_SINK@"]) ?}else{pa(&["get-source-volume","@DEFAULT_SOURCE@"]) ?};let t=o.split_whitespace().find(|x|x.ends_with('%')).ok_or_else(||anyhow::anyhow!("volume not found"))?;Ok(t.trim_end_matches('%').parse::<u8>()?.min(100))}
 pub fn get_levels()->anyhow::Result<AudioLevels>{let w=avail("wpctl");Ok(AudioLevels{output:vol(w,true)?,input:vol(w,false)?})}
 pub fn set_output(p:u8)->anyhow::Result<()>{if avail("wpctl"){wp(&["set-volume","@DEFAULT_AUDIO_SINK@",&format!("{}%",p)])?}else{pa(&["set-sink-volume","@DEFAULT_SINK@",&format!("{}%",p)])?}Ok(())}
 pub fn set_input(p:u8)->anyhow::Result<()>{if avail("wpctl"){wp(&["set-volume","@DEFAULT_AUDIO_SOURCE@",&format!("{}%",p)])?}else{pa(&["set-source-volume","@DEFAULT_SOURCE@",&format!("{}%",p)])?}Ok(())}
 pub fn toggle_mic_mute()->anyhow::Result<()>{if avail("wpctl"){wp(&["set-mute","@DEFAULT_AUDIO_SOURCE@","toggle"])?}else{pa(&["set-source-mute","@DEFAULT_SOURCE@","toggle"])?}Ok(())}
 pub fn toggle_output_mute()->anyhow::Result<()>{if avail("wpctl"){wp(&["set-mute","@DEFAULT_AUDIO_SINK@","toggle"])?}else{pa(&["set-sink-mute","@DEFAULT_SINK@","toggle"])?}Ok(())}

 pub fn get_audio_devices() -> anyhow::Result<Vec<AudioDevice>> {
   let mut devices = Vec::new();
   let use_wpctl = avail("wpctl");

   if use_wpctl {
     // Get output devices with wpctl
     if let Ok(output) = wp(&["status"]) {
       for line in output.lines() {
         if line.contains("Audio") && (line.contains("Sink") || line.contains("Source")) {
           if let Some(name) = line.split('.').nth(1) {
             let is_output = line.contains("Sink");
             devices.push(AudioDevice {
               id: name.trim().to_string(),
               name: name.trim().to_string(),
               is_output,
               is_input: !is_output,
             });
           }
         }
       }
     }
   } else {
     // Fallback to pactl
     if let Ok(output) = pa(&["list-sinks"]) {
       for line in output.lines() {
         if line.starts_with("Name:") {
           let name = line.split(':').nth(1).unwrap_or("").trim();
           devices.push(AudioDevice {
             id: name.to_string(),
             name: name.to_string(),
             is_output: true,
             is_input: false,
           });
         }
       }
     }
     if let Ok(output) = pa(&["list-sources"]) {
       for line in output.lines() {
         if line.starts_with("Name:") {
           let name = line.split(':').nth(1).unwrap_or("").trim();
           devices.push(AudioDevice {
             id: name.to_string(),
             name: name.to_string(),
             is_output: false,
             is_input: true,
           });
         }
       }
     }
   }

   Ok(devices)
 }

 pub fn set_default_output_device(device_id: &str) -> anyhow::Result<()> {
   if avail("wpctl") {
     wp(&["set-default", device_id])?;
   } else {
     pa(&["set-default-sink", device_id])?;
   }
   Ok(())
 }

 pub fn set_default_input_device(device_id: &str) -> anyhow::Result<()> {
   if avail("wpctl") {
     wp(&["set-default", device_id])?;
   } else {
     pa(&["set-default-source", device_id])?;
   }
   Ok(())
 }
}

#[cfg(target_os="macos")]
mod platform {
 use super::{AudioLevels, AudioDevice}; use std::process::Command;
 mod coreaudio_mute { include!("system_audio_macos.rs"); }
 fn osa(s:&str)->anyhow::Result<String>{let o=Command::new("osascript").args(["-e",s]).output()?;if !o.status.success(){return Err(anyhow::anyhow!(String::from_utf8_lossy(&o.stderr).trim().to_string()))}Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())}
 fn parse(s:String)->anyhow::Result<u8>{Ok(s.parse::<u8>()?.min(100))}
 pub fn get_levels()->anyhow::Result<AudioLevels>{Ok(AudioLevels{output:parse(osa("output volume of (get volume settings)")?)?,input:parse(osa("input volume of (get volume settings)")?)?})}
 pub fn set_output(p:u8)->anyhow::Result<()>{osa(&format!("set volume output volume {}",p))?;Ok(())}
 pub fn set_input(p:u8)->anyhow::Result<()>{osa(&format!("set volume input volume {}",p))?;Ok(())}

 pub fn get_audio_devices() -> anyhow::Result<Vec<AudioDevice>> {
   let mut devices = Vec::new();

   // Get output devices
   let script = r#"
     tell application "System Events"
       set outputDevices to every audio output device
       set deviceList to {}
       repeat with dev in outputDevices
         set end of deviceList to {name: name of dev, id: id of dev}
       end repeat
       return deviceList
     end tell
   "#;

   if let Ok(output) = osa(script) {
     // Parse the output (simplified parsing)
     for line in output.lines() {
       if line.contains("name:") {
         let name = line.split(':').nth(1).unwrap_or("").trim();
         let id = line.split(':').nth(2).unwrap_or("").trim();
         devices.push(AudioDevice {
           id: id.to_string(),
           name: name.to_string(),
           is_output: true,
           is_input: false,
         });
       }
     }
   }

   // Get input devices
   let script = r#"
     tell application "System Events"
       set inputDevices to every audio input device
       set deviceList to {}
       repeat with dev in inputDevices
         set end of deviceList to {name: name of dev, id: id of dev}
       end repeat
       return deviceList
     end tell
   "#;

   if let Ok(output) = osa(script) {
     for line in output.lines() {
       if line.contains("name:") {
         let name = line.split(':').nth(1).unwrap_or("").trim();
         let id = line.split(':').nth(2).unwrap_or("").trim();
         devices.push(AudioDevice {
           id: id.to_string(),
           name: name.to_string(),
           is_output: false,
           is_input: true,
         });
       }
     }
   }

   Ok(devices)
 }

 pub fn set_default_output_device(device_id: &str) -> anyhow::Result<()> {
   let script = format!(r#"
     tell application "System Events"
       set currentDevice to audio output device whose id is "{}"
       set current output volume of currentDevice to output volume of currentDevice
     end tell
   "#, device_id);
   osa(&script)?;
   Ok(())
 }

 pub fn set_default_input_device(device_id: &str) -> anyhow::Result<()> {
   let script = format!(r#"
     tell application "System Events"
       set currentDevice to audio input device whose id is "{}"
       set current input volume of currentDevice to input volume of currentDevice
     end tell
   "#, device_id);
   osa(&script)?;
   Ok(())
 }

 pub fn toggle_mic_mute()->anyhow::Result<()>{coreaudio_mute::toggle_input_mute()}
 pub fn toggle_output_mute()->anyhow::Result<()>{osa("set volume output muted not (output muted of (get volume settings))")?;Ok(())}
}

#[cfg(not(any(target_os="windows",target_os="linux",target_os="macos")))]
mod platform { use super::AudioLevels; pub fn get_levels()->anyhow::Result<AudioLevels>{Err(anyhow::anyhow!("System audio controls are not implemented for this platform"))} pub fn set_output(_:u8)->anyhow::Result<()>{Err(anyhow::anyhow!("System audio controls are not implemented for this platform"))} pub fn set_input(_:u8)->anyhow::Result<()>{Err(anyhow::anyhow!("System audio controls are not implemented for this platform"))} pub fn toggle_mic_mute()->anyhow::Result<()>{Err(anyhow::anyhow!("System microphone controls are not implemented for this platform"))} pub fn toggle_output_mute()->anyhow::Result<()>{Err(anyhow::anyhow!("System output mute is not implemented for this platform"))} }
