// Add this to your existing src/config.rs

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrayConfig {
    /// Show battery percentage in tray icon (Windows only)
    pub show_battery_percentage: bool,
    /// Use monochrome/symbolic icons (Linux only)
    pub monochrome_icons: bool,
    /// Tray icon refresh interval in seconds
    pub refresh_interval_secs: u64,
    /// Color thresholds for battery icon
    pub color_high: [u8; 3],      // RGB for 50-100%
    pub color_medium: [u8; 3],    // RGB for 20-49%
    pub color_low: [u8; 3],       // RGB for 0-19%
    pub color_charging: [u8; 3],  // RGB when charging
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            show_battery_percentage: true,
            monochrome_icons: false,
            refresh_interval_secs: 3,
            color_high: [96, 196, 106],      // Green
            color_medium: [245, 166, 35],    // Orange
            color_low: [220, 90, 90],        // Red
            color_charging: [245, 216, 64],   // Yellow
        }
    }
}

// Add to your main Config struct:
// #[serde(default)]
// pub tray: TrayConfig,
