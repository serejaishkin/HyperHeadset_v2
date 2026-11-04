use image::{Rgba, RgbaImage};

use super::config::{TrayIconConfig, OFFLINE_PERCENT};

/// Vertical battery that fills the whole icon: the filled height is the charge, the fill uses the status color,
/// the frame and the bolt (charging) are light so they read on dark and light taskbars; only the status (BG) color is configurable.
/// No digits, so it stays readable at 16x16.
pub fn generate_battery_gauge_rgba(
    config: &TrayIconConfig,
    percent: u8,
    charging: bool,
) -> (Vec<u8>, u32, u32) {
    let size = config.size.max(16);
    let s = size as f32;
    let mut img = RgbaImage::new(size, size);

    let scheme = config.scheme_for(percent, charging);
    let offline = percent == OFFLINE_PERCENT;
    let fill_c = Rgba(config.status_color(percent, charging));
    let frame_c = if offline { Rgba(scheme.fg) } else { Rgba([240, 240, 240, 255]) };
    let mark_c = frame_c;
    let track_c = Rgba([0, 0, 0, 150]);

    // Geometry (in pixels).
    let t = ((s / 10.0).round() as i32).max(1); // frame thickness
    let bx0 = (s * 0.20).round() as i32;
    let bx1 = (s * 0.80).round() as i32; // exclusive
    let by0 = (s * 0.14).round() as i32;
    let by1 = size as i32 - 1; // exclusive
    let cx0 = (s * 0.38).round() as i32;
    let cx1 = (s * 0.62).round() as i32;
    let cy0 = 0;
    let cy1 = by0;

    let inner_x0 = bx0 + t;
    let inner_x1 = bx1 - t;
    let inner_y0 = by0 + t;
    let inner_y1 = by1 - t;
    let inner_h = (inner_y1 - inner_y0).max(1);
    let level = if offline { 0 } else { percent.min(100) as i32 };
    let mut fill_h = (inner_h * level + 50) / 100;
    if !offline && level > 0 && fill_h == 0 { fill_h = 1; }
    let fill_top = inner_y1 - fill_h;

    let put = |x: i32, y: i32, c: Rgba<u8>, img: &mut RgbaImage| {
        if x >= 0 && y >= 0 && (x as u32) < size && (y as u32) < size { img.put_pixel(x as u32, y as u32, c); }
    };

    for y in cy0..cy1 { for x in cx0..cx1 { put(x, y, frame_c, &mut img); } }
    for y in by0..by1 {
        for x in bx0..bx1 {
            let in_frame = x < inner_x0 || x >= inner_x1 || y < inner_y0 || y >= inner_y1;
            let c = if in_frame { frame_c } else if y >= fill_top { fill_c } else { track_c };
            put(x, y, c, &mut img);
        }
    }

    let w = (inner_x1 - inner_x0) as f32;
    let h = inner_h as f32;
    let (ox, oy) = (inner_x0 as f32, inner_y0 as f32);
    if charging && !offline {
        // Lightning bolt polygon in inner-box fractions.
        let poly = [(0.72, 0.0), (0.06, 0.56), (0.46, 0.56), (0.28, 1.0), (0.94, 0.42), (0.54, 0.42)];
        for y in inner_y0..inner_y1 {
            for x in inner_x0..inner_x1 {
                let px = (x as f32 + 0.5 - ox) / w;
                let py = (y as f32 + 0.5 - oy) / h;
                if inside(&poly, px, py) { put(x, y, mark_c, &mut img); }
            }
        }
    } else if offline {
        // Cross in the middle of the empty body.
        let side = w.min(h) * 0.7;
        let (cx, cy) = (ox + w / 2.0, oy + h / 2.0);
        let thick = (s * 0.07).max(1.0);
        for y in inner_y0..inner_y1 {
            for x in inner_x0..inner_x1 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if dx.abs() <= side / 2.0 && dy.abs() <= side / 2.0
                    && ((dx - dy).abs() < thick || (dx + dy).abs() < thick)
                {
                    put(x, y, fill_c, &mut img);
                }
            }
        }
    }

    (img.into_raw(), size, size)
}

fn inside(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut c = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi { c = !c; }
        j = i;
    }
    c
}
