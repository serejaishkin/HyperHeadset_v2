use windows::{
    core::GUID,
    Win32::{
        Media::Audio::{
            eConsole, eRender, eCapture,
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

    // ========== MASTER (render) ==========
    pub fn get_master_volume(&self) -> Option<f32> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
            let level = endpoint.GetMasterVolumeLevelScalar().ok()?;
            Some(level * 100.0)
        }
    }

    pub fn set_master_volume(&self, percent: f32) -> bool {
        unsafe {
            let enumerator: Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL);
            let Ok(enumerator) = enumerator else { return false; };
            let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) else { return false; };
            let endpoint: Result<IAudioEndpointVolume, _> = device.Activate(CLSCTX_ALL, None);
            let Ok(endpoint) = endpoint else { return false; };
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
            let enumerator: Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL);
            let Ok(enumerator) = enumerator else { return false; };
            let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) else { return false; };
            let endpoint: Result<IAudioEndpointVolume, _> = device.Activate(CLSCTX_ALL, None);
            let Ok(endpoint) = endpoint else { return false; };
            endpoint.SetMute(muted, &GUID::zeroed()).is_ok()
        }
    }

    // ========== MICROPHONE (capture) ==========
    pub fn get_microphone_volume(&self) -> Option<f32> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole).ok()?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
            let level = endpoint.GetMasterVolumeLevelScalar().ok()?;
            Some(level * 100.0)
        }
    }

    pub fn set_microphone_volume(&self, percent: f32) -> bool {
        unsafe {
            let enumerator: Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL);
            let Ok(enumerator) = enumerator else { return false; };
            let Ok(device) = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole) else { return false; };
            let endpoint: Result<IAudioEndpointVolume, _> = device.Activate(CLSCTX_ALL, None);
            let Ok(endpoint) = endpoint else { return false; };
            let scalar = (percent / 100.0).clamp(0.0, 1.0);
            endpoint.SetMasterVolumeLevelScalar(scalar, &GUID::zeroed()).is_ok()
        }
    }

    pub fn get_microphone_mute(&self) -> Option<bool> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole).ok()?;
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
            let muted = endpoint.GetMute().ok()?;
            Some(muted.as_bool())
        }
    }

    pub fn set_microphone_mute(&self, muted: bool) -> bool {
        unsafe {
            let enumerator: Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL);
            let Ok(enumerator) = enumerator else { return false; };
            let Ok(device) = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole) else { return false; };
            let endpoint: Result<IAudioEndpointVolume, _> = device.Activate(CLSCTX_ALL, None);
            let Ok(endpoint) = endpoint else { return false; };
            endpoint.SetMute(muted, &GUID::zeroed()).is_ok()
        }
    }
}