//! System-level audio EQ integration
//! Since HyperX headsets do not expose EQ via HID, we control EQ at the OS level.

use crate::config::AudioConfig;

pub mod debounce;
pub use debounce::DebouncedEQ;

#[cfg(target_os = "macos")]
pub mod macos_eqmac;

pub trait AudioBackend: Send + Sync {
    fn apply_eq(&self, bands: &[f32; 10]) -> anyhow::Result<()>;
    fn save_preset(&self, name: &str, bands: &[f32; 10]) -> anyhow::Result<()>;
    fn load_preset(&self, name: &str) -> anyhow::Result<[f32; 10]>;
    fn list_presets(&self) -> Vec<String>;
    fn is_available(&self) -> bool;
}

pub struct AudioManager {
    backend: Box<dyn AudioBackend>,
}

impl AudioManager {
    pub fn new() -> Self {
        #[cfg(target_os = "windows")]
        let backend: Box<dyn AudioBackend> = Box::new(WindowsAPOBackend);

        #[cfg(target_os = "linux")]
        let backend: Box<dyn AudioBackend> = Box::new(LinuxPipewireBackend);

        #[cfg(target_os = "macos")]
        let backend: Box<dyn AudioBackend> = Box::new(MacOSCoreAudioBackend);

        Self { backend }
    }

    pub fn apply_preset(&self, config: &AudioConfig) -> anyhow::Result<()> {
        if config.system_eq_enabled {
            self.backend.apply_eq(&config.eq_bands)?;
        }
        Ok(())
    }

    pub fn backend(&self) -> &dyn AudioBackend {
        self.backend.as_ref()
    }
}

// ===== Windows: Equalizer APO =====
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "windows")]
pub struct WindowsAPOBackend;

#[cfg(target_os = "windows")]
impl AudioBackend for WindowsAPOBackend {
    fn apply_eq(&self, bands: &[f32; 10]) -> anyhow::Result<()> {
        windows::apply_eq_bands(bands)
    }

    fn save_preset(&self, name: &str, bands: &[f32; 10]) -> anyhow::Result<()> {
        windows::save_preset(name, bands)
    }

    fn load_preset(&self, name: &str) -> anyhow::Result<[f32; 10]> {
        windows::load_preset(name)
    }

    fn list_presets(&self) -> Vec<String> {
        windows::list_presets()
    }

    fn is_available(&self) -> bool {
        windows::is_apo_installed()
    }
}

// ===== Linux: PipeWire / EasyEffects =====
#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "linux")]
pub struct LinuxPipewireBackend;

#[cfg(target_os = "linux")]
impl AudioBackend for LinuxPipewireBackend {
    fn apply_eq(&self, bands: &[f32; 10]) -> anyhow::Result<()> {
        linux::apply_eq_bands(bands)
    }

    fn save_preset(&self, name: &str, bands: &[f32; 10]) -> anyhow::Result<()> {
        linux::save_preset(name, bands)
    }

    fn load_preset(&self, name: &str) -> anyhow::Result<[f32; 10]> {
        linux::load_preset(name)
    }

    fn list_presets(&self) -> Vec<String> {
        linux::list_presets()
    }

    fn is_available(&self) -> bool {
        linux::is_easyeffects_available() || linux::is_pipewire_running()
    }
}

// ===== macOS: eqMac =====
#[cfg(target_os = "macos")]
pub struct MacOSCoreAudioBackend;

#[cfg(target_os = "macos")]
impl AudioBackend for MacOSCoreAudioBackend {
    fn apply_eq(&self, bands: &[f32; 10]) -> anyhow::Result<()> {
        // eqMac is async, so we block_on for simplicity in sync context
        // In real app, use tokio runtime
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(macos_eqmac::apply_eq_bands(bands))
    }

    fn save_preset(&self, name: &str, bands: &[f32; 10]) -> anyhow::Result<()> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(macos_eqmac::save_preset(name, bands))
    }

    fn load_preset(&self, name: &str) -> anyhow::Result<[f32; 10]> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(macos_eqmac::load_preset(name))
    }

    fn list_presets(&self) -> Vec<String> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(macos_eqmac::list_presets()).unwrap_or_default()
    }

    fn is_available(&self) -> bool {
        macos_eqmac::is_eqmac_running()
    }
}
