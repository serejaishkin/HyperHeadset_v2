pub struct MacOSVolume;

impl MacOSVolume {
    pub fn new() -> Self { Self }
    pub fn get_master_volume(&self) -> Option<f32> { None }
    pub fn set_master_volume(&self, _vol: f32) {}
}