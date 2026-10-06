pub struct MacOSVolume;

impl MacOSVolume {
    pub fn new() -> Self { Self }
    pub fn get_master_volume(&self) -> Option<f32> { None }
    pub fn set_master_volume(&self, _vol: f32) {}
    pub fn get_microphone_volume(&self) -> Option<f32> { None }
    pub fn set_microphone_volume(&self, _level: f32) {}
    pub fn get_microphone_mute(&self) -> Option<bool> { None }
    pub fn set_microphone_mute(&self, _muted: bool) {}
}