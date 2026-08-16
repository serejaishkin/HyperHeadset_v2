#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::{
    eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator,
};

pub struct WindowsVolume {
    #[cfg(target_os = "windows")]
    endpoint_volume: Option<IAudioEndpointVolume>,
}

impl WindowsVolume {
    #[cfg(target_os = "windows")]
    pub fn new() -> Self {
        unsafe {
            let enumerator: IMMDeviceEnumerator = match CoCreateInstance(
                &MMDeviceEnumerator, None, CLSCTX_ALL,
            ) {
                Ok(e) => e,
                Err(_) => return Self { endpoint_volume: None },
            };
            let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                Ok(d) => d,
                Err(_) => return Self { endpoint_volume: None },
            };
            let endpoint_volume = match device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                Ok(v) => Some(v),
                Err(_) => None,
            };
            Self { endpoint_volume }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new() -> Self { Self {} }

    #[cfg(target_os = "windows")]
    pub fn get_master_volume(&self) -> Option<f32> {
        unsafe {
            let vol = self.endpoint_volume.as_ref()?;
            let mut level = 0.0f32;
            vol.GetMasterVolumeLevelScalar(&mut level).ok()?;
            Some(level)
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn get_master_volume(&self) -> Option<f32> { None }

    #[cfg(target_os = "windows")]
    pub fn set_master_volume(&self, level: f32) {
        unsafe {
            if let Some(vol) = self.endpoint_volume.as_ref() {
                let _ = vol.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), &std::ptr::null());
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_master_volume(&self, _level: f32) {}
}

#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};