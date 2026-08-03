use windows::{
    core::GUID,
    Win32::{
        Media::Audio::{
            eConsole, eRender,
            Endpoints::IAudioEndpointVolume,
            IMMDeviceEnumerator, MMDeviceEnumerator,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
        },
    },
};

pub struct WindowsVolume;

impl WindowsVolume {
    pub fn new() -> Self {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        Self
    }

    /// 0.0 .. 100.0
    pub fn get_master_volume(&self) -> Option<f32> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
            let level = endpoint.GetMasterVolumeLevelScalar().ok()?;
            Some(level * 100.0)
        }
    }

    /// 0.0 .. 100.0
    pub fn set_master_volume(&self, percent: f32) -> bool {
        unsafe {
            let Ok(enumerator): Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) else {
                return false;
            };
            let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) else {
                return false;
            };
            let Ok(endpoint): Result<IAudioEndpointVolume, _> = device.Activate(CLSCTX_ALL, None) else {
                return false;
            };
            let scalar = (percent / 100.0).clamp(0.0, 1.0);
            endpoint.SetMasterVolumeLevelScalar(scalar, &GUID::zeroed()).is_ok()
        }
    }

    pub fn get_mute(&self) -> Option<bool> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
            let muted = endpoint.GetMute().ok()?;
            Some(muted.as_bool())
        }
    }

    pub fn set_mute(&self, muted: bool) -> bool {
        unsafe {
            let Ok(enumerator): Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) else {
                return false;
            };
            let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) else {
                return false;
            };
            let Ok(endpoint): Result<IAudioEndpointVolume, _> = device.Activate(CLSCTX_ALL, None) else {
                return false;
            };
            endpoint.SetMute(muted, &GUID::zeroed()).is_ok()
        }
    }
}
