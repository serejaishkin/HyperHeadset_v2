//! Moves the audio streams of running applications to a given output device, the same way
//! Windows' "Volume mixer / App volume and device preferences" does (per-app output device).
//!
//! Changing the *default* device only affects streams that follow the default. Players that pinned
//! a device (or opened their stream a moment late) can stay on the old one and go silent while system
//! sounds work. After every default-device switch we find such sessions and move them.
//!
//! Uses the undocumented `Windows.Media.Internal.AudioPolicyConfig` factory (same one EarTrumpet
//! uses). Both known interface variants have the same vtable layout; only the IID differs (21H2+
//! vs older). Every failure is logged and non-fatal.

/// Moves every audio session that is not on `target_device_id` (an MMDevice id) to it.
/// Returns how many applications were moved.
#[cfg(target_os = "windows")]
pub fn move_streams_to(target_device_id: &str) -> anyhow::Result<usize> {
    imp::move_streams_to(target_device_id)
}

#[cfg(not(target_os = "windows"))]
pub fn move_streams_to(_target_device_id: &str) -> anyhow::Result<usize> {
    Ok(0)
}

#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;
    use windows::core::{Interface, GUID, HRESULT, HSTRING};
    use windows::Win32::Media::Audio::{
        eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator,
        MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED};

    #[link(name = "combase", kind = "raw-dylib")]
    extern "system" {
        fn RoGetActivationFactory(class_id: *mut c_void, iid: *const GUID, factory: *mut *mut c_void) -> HRESULT;
    }

    // IAudioPolicyConfigFactory variants (differ only by IID; identical vtable layout).
    const IID_21H2: GUID = GUID::from_u128(0xab3d4648_e242_459f_b02f_541c70306324);
    const IID_DOWNLEVEL: GUID = GUID::from_u128(0x2a59116d_6c4f_45e0_a74f_707e3fef9258);
    // IUnknown (3) + IInspectable (3) + 19 unused methods, then Set/Get/ClearAll.
    const SLOT_SET_PERSISTED: usize = 25;
    const SLOT_GET_PERSISTED: usize = 26;
    const SLOT_RELEASE: usize = 2;

    const E_RENDER: i32 = 0;
    const ROLE_CONSOLE: i32 = 0;
    const ROLE_MULTIMEDIA: i32 = 1;

    type SetFn = unsafe extern "system" fn(*mut c_void, u32, i32, i32, *mut c_void) -> HRESULT;
    type GetFn = unsafe extern "system" fn(*mut c_void, u32, i32, i32, *mut *mut c_void) -> HRESULT;
    type ReleaseFn = unsafe extern "system" fn(*mut c_void) -> u32;

    struct Factory(*mut c_void);
    impl Factory {
        unsafe fn open() -> anyhow::Result<Factory> {
            let class = HSTRING::from("Windows.Media.Internal.AudioPolicyConfig");
            let raw_class: *mut c_void = std::mem::transmute_copy(&class);
            let mut last = HRESULT(0);
            for iid in [&IID_21H2, &IID_DOWNLEVEL] {
                let mut f: *mut c_void = std::ptr::null_mut();
                let hr = RoGetActivationFactory(raw_class, iid, &mut f);
                if hr.is_ok() && !f.is_null() {
                    return Ok(Factory(f));
                }
                last = hr;
            }
            Err(anyhow::anyhow!("AudioPolicyConfig factory not available (hr=0x{:08X})", last.0 as u32))
        }

        /// True when the app already has a per-app output chosen by the user (Windows "App volume and
        /// device preferences"); we must neither override nor clear such a choice.
        unsafe fn has_pin(&self, pid: u32, flow: i32, role: i32) -> bool {
            let vtbl = *(self.0 as *const *const usize);
            let f: GetFn = std::mem::transmute(*vtbl.add(SLOT_GET_PERSISTED));
            let mut out: *mut c_void = std::ptr::null_mut();
            let hr = f(self.0, pid, flow, role, &mut out);
            if out.is_null() {
                return false;
            }
            // Take ownership of the returned HSTRING so it is freed.
            let h: HSTRING = std::mem::transmute(out);
            hr.is_ok() && !h.is_empty()
        }

        /// `device_id` None/empty clears the per-app preference (stream follows the default again).
        unsafe fn set(&self, pid: u32, flow: i32, role: i32, hstr_device: Option<&str>) -> HRESULT {
            let vtbl = *(self.0 as *const *const usize);
            let f: SetFn = std::mem::transmute(*vtbl.add(SLOT_SET_PERSISTED));
            match hstr_device {
                Some(id) => {
                    let h = HSTRING::from(id);
                    let raw: *mut c_void = std::mem::transmute_copy(&h);
                    f(self.0, pid, flow, role, raw)
                }
                None => f(self.0, pid, flow, role, std::ptr::null_mut()),
            }
        }
    }
    impl Drop for Factory {
        fn drop(&mut self) {
            unsafe {
                let vtbl = *(self.0 as *const *const usize);
                let r: ReleaseFn = std::mem::transmute(*vtbl.add(SLOT_RELEASE));
                r(self.0);
            }
        }
    }

    fn policy_device_id(mmdevice_id: &str) -> String {
        format!(r"\\?\SWD#MMDEVAPI#{}#{{e6327cad-dcec-4949-ae8a-991e976a79d2}}", mmdevice_id)
    }

    /// PIDs of apps that have a live session on an active render endpoint other than `target`.
    unsafe fn pids_not_on(en: &IMMDeviceEnumerator, target: &str) -> anyhow::Result<Vec<u32>> {
        let me = std::process::id();
        let mut pids: Vec<u32> = Vec::new();
        let devices = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        for i in 0..devices.GetCount()? {
            let dev = match devices.Item(i) { Ok(d) => d, Err(_) => continue };
            let id = match dev.GetId().ok().and_then(|p| {
                let s = p.to_string().ok();
                CoTaskMemFree(Some(p.0 as *const c_void));
                s
            }) { Some(s) => s, None => continue };
            if id == target {
                continue;
            }
            let mgr: IAudioSessionManager2 = match dev.Activate(CLSCTX_ALL, None) { Ok(m) => m, Err(_) => continue };
            let sessions = match mgr.GetSessionEnumerator() { Ok(s) => s, Err(_) => continue };
            for j in 0..sessions.GetCount().unwrap_or(0) {
                let ctl = match sessions.GetSession(j) { Ok(c) => c, Err(_) => continue };
                // Only sessions that are playing right now (1 = AudioSessionStateActive); idle or
                // expired sessions on other devices must not be touched.
                if ctl.GetState().map(|s| s.0 != 1).unwrap_or(true) {
                    continue;
                }
                let ctl2: IAudioSessionControl2 = match ctl.cast() { Ok(c) => c, Err(_) => continue };
                if let Ok(pid) = ctl2.GetProcessId() {
                    if pid != 0 && pid != me && !pids.contains(&pid) {
                        pids.push(pid);
                    }
                }
            }
        }
        Ok(pids)
    }

    pub fn move_streams_to(target: &str) -> anyhow::Result<usize> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let pids = pids_not_on(&en, target)?;
            if pids.is_empty() {
                return Ok(0);
            }
            let factory = Factory::open()?;
            let dev = policy_device_id(target);
            let mut moved = 0usize;
            // Apps the user pinned to a device themselves are left alone (and their pin is never cleared).
            let pids: Vec<u32> = pids
                .into_iter()
                .filter(|pid| {
                    let pinned = factory.has_pin(*pid, E_RENDER, ROLE_CONSOLE);
                    if pinned {
                        log::info!("[Audio] pid {} has its own device chosen in Windows settings, not moving it", pid);
                    }
                    !pinned
                })
                .collect();
            for pid in &pids {
                let mut ok = true;
                for role in [ROLE_CONSOLE, ROLE_MULTIMEDIA] {
                    let hr = factory.set(*pid, E_RENDER, role, Some(&dev));
                    if hr.is_err() {
                        log::warn!("[Audio] Move stream of pid {} failed: 0x{:08X}", pid, hr.0 as u32);
                        ok = false;
                        break;
                    }
                }
                if ok {
                    moved += 1;
                    log::info!("[Audio] Moved stream of pid {} to the current output", pid);
                }
            }
            // The streams have moved; drop the per-app pin so the app follows the default again
            // (otherwise it would stay pinned to this device when it later disappears).
            std::thread::sleep(std::time::Duration::from_millis(400));
            for pid in &pids {
                for role in [ROLE_CONSOLE, ROLE_MULTIMEDIA] {
                    let _ = factory.set(*pid, E_RENDER, role, None);
                }
            }
            Ok(moved)
        }
    }
}
