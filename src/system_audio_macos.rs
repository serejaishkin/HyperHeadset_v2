//! Native CoreAudio input mute support for macOS.
use std::ffi::c_void;
type AudioDeviceId = u32;
type OSStatus = i32;
const SYSTEM_OBJECT: AudioDeviceId = 1;
const GLOBAL: u32 = u32::from_be_bytes(*b"glob");
const INPUT: u32 = u32::from_be_bytes(*b"inpt");
const MAIN: u32 = 0;
const DEFAULT_INPUT: u32 = u32::from_be_bytes(*b"dIn ");
const MUTE: u32 = u32::from_be_bytes(*b"mute");
#[repr(C)] struct Address { selector: u32, scope: u32, element: u32 }
#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectGetPropertyData(object: AudioDeviceId, address: *const Address, qualifier_size: u32, qualifier: *const c_void, data_size: *mut u32, data: *mut c_void) -> OSStatus;
    fn AudioObjectSetPropertyData(object: AudioDeviceId, address: *const Address, qualifier_size: u32, qualifier: *const c_void, data_size: u32, data: *const c_void) -> OSStatus;
}
fn check(status: OSStatus, what: &str) -> anyhow::Result<()> { if status == 0 { Ok(()) } else { Err(anyhow::anyhow!("CoreAudio {} failed: OSStatus {}", what, status)) } }
fn default_input_device() -> anyhow::Result<AudioDeviceId> {
    let address = Address { selector: DEFAULT_INPUT, scope: GLOBAL, element: MAIN };
    let mut device = 0u32; let mut size = 4u32;
    unsafe { check(AudioObjectGetPropertyData(SYSTEM_OBJECT, &address, 0, std::ptr::null(), &mut size, &mut device as *mut _ as *mut _), "get default input device")?; }
    if device == 0 { return Err(anyhow::anyhow!("No default input device")); }
    Ok(device)
}
pub fn toggle_input_mute() -> anyhow::Result<()> {
    let device = default_input_device()?;
    let address = Address { selector: MUTE, scope: INPUT, element: MAIN };
    let mut muted = 0u32; let mut size = 4u32;
    unsafe {
        check(AudioObjectGetPropertyData(device, &address, 0, std::ptr::null(), &mut size, &mut muted as *mut _ as *mut _), "read input mute")?;
        let new_value: u32 = if muted == 0 { 1 } else { 0 };
        check(AudioObjectSetPropertyData(device, &address, 0, std::ptr::null(), 4, &new_value as *const _ as *const _), "set input mute")?;
    }
    Ok(())
}
