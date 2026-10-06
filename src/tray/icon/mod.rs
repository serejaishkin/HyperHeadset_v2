mod config;
mod big;
mod digits;

pub use config::{TrayIconConfig, TrayIconColors, IconColors, TrayIconMode};
pub use big::generate_big_digits_rgba;
pub use digits::generate_battery_icon_rgba;

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
