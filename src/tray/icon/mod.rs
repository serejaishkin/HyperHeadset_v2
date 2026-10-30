mod config;
mod big;
mod digits;

pub use config::{TrayIconConfig, TrayIconColors, IconColors, TrayIconMode};
pub use big::generate_big_digits_rgba;
pub use digits::generate_battery_icon_rgba;

/// Render the tray icon for the configured mode:
/// Big    = large outlined digits that fill the icon on a transparent background;
/// Digits = small digits on a coloured background.
pub fn generate_tray_icon_rgba(
    config: &TrayIconConfig,
    percent: u8,
    charging: bool,
) -> (Vec<u8>, u32, u32) {
    match config.mode {
        TrayIconMode::Big => generate_big_digits_rgba(percent, charging, config),
        TrayIconMode::Digits => generate_battery_icon_rgba(config, percent, charging),
    }
}

pub fn generate_battery_icon_png(
    config: &TrayIconConfig,
    percent: u8,
    charging: bool,
) -> anyhow::Result<Vec<u8>> {
    let (rgba, w, h) = generate_battery_icon_rgba(config, percent, charging);
    let img = image::RgbaImage::from_raw(w, h, rgba)
        .ok_or_else(|| anyhow::anyhow!("Invalid image buffer"))?;
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)?;
    Ok(buf)
}
