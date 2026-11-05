mod config;
mod big;
mod battery;

pub use config::{OFFLINE_PERCENT, TrayIconConfig, TrayIconColors, IconColors, TrayIconMode};
pub use big::generate_big_digits_rgba;
pub use battery::generate_battery_gauge_rgba;

/// Render the tray icon for the configured mode:
/// Big     = large outlined digits that fill the icon on a transparent background;
/// Battery = battery gauge filled by charge (readable at 16px, no digits).
pub fn generate_tray_icon_rgba(
    config: &TrayIconConfig,
    percent: u8,
    charging: bool,
) -> (Vec<u8>, u32, u32) {
    match config.mode {
        TrayIconMode::Big => generate_big_digits_rgba(percent, charging, config),
        TrayIconMode::Battery => generate_battery_gauge_rgba(config, percent, charging),
    }
}

pub fn generate_battery_icon_png(
    config: &TrayIconConfig,
    percent: u8,
    charging: bool,
) -> anyhow::Result<Vec<u8>> {
    let (rgba, w, h) = generate_battery_gauge_rgba(config, percent, charging);
    let img = image::RgbaImage::from_raw(w, h, rgba)
        .ok_or_else(|| anyhow::anyhow!("Invalid image buffer"))?;
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)?;
    Ok(buf)
}

/// Convert RGBA8 to the ARGB32 (network byte order: A,R,G,B) layout used by StatusNotifierItem.
pub fn rgba_to_argb32(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    for px in rgba.chunks_exact(4) {
        out.extend_from_slice(&[px[3], px[0], px[1], px[2]]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(mode: TrayIconMode, size: u32, smooth: bool) -> TrayIconConfig {
        let mut c = TrayIconConfig::default();
        c.mode = mode;
        c.size = size;
        c.smooth_colors = smooth;
        c
    }

    #[test]
    fn every_mode_size_and_percent_renders_without_panic() {
        for mode in [TrayIconMode::Big, TrayIconMode::Battery] {
            for size in [16u32, 20, 24, 32, 48, 64] {
                for smooth in [false, true] {
                    let c = cfg(mode.clone(), size, smooth);
                    let mut pcts: Vec<u8> = (0..=100).collect();
                    pcts.push(OFFLINE_PERCENT);
                    for p in pcts {
                        for charging in [false, true] {
                            let (buf, w, h) = generate_tray_icon_rgba(&c, p, charging);
                            assert_eq!((w, h), (size, size));
                            assert_eq!(buf.len(), (w * h * 4) as usize);
                            assert!(buf.chunks(4).any(|px| px[3] > 0), "icon is fully transparent: {:?} {} {} {}", mode, size, p, charging);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn offline_icon_differs_from_last_known_percent() {
        for mode in [TrayIconMode::Big, TrayIconMode::Battery] {
            let c = cfg(mode, 32, false);
            let a = generate_tray_icon_rgba(&c, 87, false).0;
            let b = generate_tray_icon_rgba(&c, OFFLINE_PERCENT, false).0;
            assert_ne!(a, b);
        }
    }

    #[test]
    fn thresholds_pick_the_right_status() {
        let c = TrayIconConfig::default();
        assert_eq!(c.scheme_for(OFFLINE_PERCENT, true).bg, c.colors.offline.bg);
        assert_eq!(c.scheme_for(10, true).bg, c.colors.charging.bg);
        assert_eq!(c.scheme_for(c.high_threshold + 1, false).bg, c.colors.high.bg);
        assert_eq!(c.scheme_for(c.high_threshold, false).bg, c.colors.medium.bg);
        assert_eq!(c.scheme_for(c.low_threshold, false).bg, c.colors.low.bg);
    }

    #[test]
    fn smooth_color_hits_anchors_and_stays_between() {
        let mut c = TrayIconConfig::default();
        c.smooth_colors = true;
        assert_eq!(c.status_color(0, false), c.colors.low.bg);
        assert_eq!(c.status_color(100, false), c.colors.high.bg);
        let mid = ((c.low_threshold as u16 + c.high_threshold as u16) / 2) as u8;
        assert_eq!(c.status_color(mid, false), c.colors.medium.bg);
        // charging / offline are never blended
        assert_eq!(c.status_color(50, true), c.colors.charging.bg);
        assert_eq!(c.status_color(OFFLINE_PERCENT, false), c.colors.offline.bg);
    }

    #[test]
    fn old_configs_still_load() {
        // mode = "digits" came from the removed small-digits mode
        let mut v = toml::to_string(&TrayIconConfig::default()).unwrap();
        v = v.replace("mode = \"big\"", "mode = \"digits\"");
        let c: TrayIconConfig = toml::from_str(&v).unwrap();
        assert_eq!(c.mode, TrayIconMode::Big);
    }
}
