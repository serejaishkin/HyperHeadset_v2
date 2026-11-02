use image::{Rgba, RgbaImage};

use super::config::TrayIconConfig;

/// Seven-segment masks, bit order: a(top) b(upper right) c(lower right) d(bottom) e(lower left)
/// f(upper left) g(middle).
const SEGMENTS: [u8; 10] = [
    0b0111111, // 0: a b c d e f
    0b0000110, // 1: b c
    0b1011011, // 2: a b d e g
    0b1001111, // 3: a b c d g
    0b1100110, // 4: b c f g
    0b1101101, // 5: a c d f g
    0b1111101, // 6: a c d e f g
    0b0000111, // 7: a b c
    0b1111111, // 8: all
    0b1101111, // 9: a b c d f g
];

fn fill(img: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, color: Rgba<u8>) {
    let size = img.width() as i32;
    for yy in y.max(0)..(y + h).min(size) {
        for xx in x.max(0)..(x + w).min(size) {
            img.put_pixel(xx as u32, yy as u32, color);
        }
    }
}

/// Rectangles (x, y, w, h) of the lit segments of one digit in a W x H box with stroke `t`.
fn digit_rects(d: usize, x: i32, y: i32, w: i32, h: i32, t: i32) -> Vec<(i32, i32, i32, i32)> {
    let mask = SEGMENTS[d.min(9)];
    let mid = y + (h - t) / 2;
    let half = (h + t) / 2; // a vertical segment spans from the outer edge to the middle bar
    let mut r = Vec::with_capacity(7);
    if mask & 0b0000001 != 0 { r.push((x, y, w, t)); } // a
    if mask & 0b0000010 != 0 { r.push((x + w - t, y, t, half)); } // b
    if mask & 0b0000100 != 0 { r.push((x + w - t, mid, t, y + h - mid)); } // c
    if mask & 0b0001000 != 0 { r.push((x, y + h - t, w, t)); } // d
    if mask & 0b0010000 != 0 { r.push((x, mid, t, y + h - mid)); } // e
    if mask & 0b0100000 != 0 { r.push((x, y, t, half)); } // f
    if mask & 0b1000000 != 0 { r.push((x, mid, w, t)); } // g
    r
}

/// Big tray digits: seven-segment numerals that fill the whole icon. Seven-segment shapes stay
/// legible at 16-24 px where a scaled bitmap font turns to mush; the stroke grows with the icon.
pub fn generate_big_digits_rgba(percent: u8, charging: bool, config: &TrayIconConfig) -> (Vec<u8>, u32, u32) {
    let size = config.size;
    let mut img = RgbaImage::new(size, size);

    let scheme = if charging {
        &config.colors.charging
    } else if percent > 50 {
        &config.colors.high
    } else if percent > 20 {
        &config.colors.medium
    } else {
        &config.colors.low
    };
    let fg = Rgba(scheme.fg);
    let outline = Rgba(scheme.outline);

    let chars: Vec<usize> = percent.min(100).to_string().chars().map(|c| c.to_digit(10).unwrap_or(0) as usize).collect();
    let n = chars.len() as i32;
    let s = size as i32;

    // Outline: 1 px on real tray sizes (so no glyph pixels are lost), proportional on big previews.
    let o: i32 = if config.outline_width <= 0 {
        0
    } else if s < 48 {
        1
    } else {
        (config.outline_width).max(s / 32)
    };
    let avail = (s - 2 * o).max(4);

    // Stroke ~ size/10 (1 px at 16, 2 at 24, 3 at 32...), reduced until n digits fit.
    let mut t = (s / 10).max(1);
    let (mut gap, mut w);
    loop {
        gap = t.max(1);
        w = (avail - (n - 1) * gap) / n;
        if w >= 2 * t + 1 || t == 1 {
            break;
        }
        t -= 1;
    }
    // A lone digit must not stretch across the whole icon: keep a normal digit proportion.
    let w = w.min(avail * 3 / 5).max(3);
    let h = avail;
    let total_w = n * w + (n - 1) * gap;
    let x0 = (s - total_w) / 2;
    let y0 = (s - h) / 2;

    let mut all: Vec<(i32, i32, i32, i32)> = Vec::new();
    for (i, d) in chars.iter().enumerate() {
        let x = x0 + i as i32 * (w + gap);
        all.extend(digit_rects(*d, x, y0, w, h, t));
    }
    if o > 0 {
        for &(x, y, rw, rh) in &all {
            fill(&mut img, x - o, y - o, rw + 2 * o, rh + 2 * o, outline);
        }
    }
    for &(x, y, rw, rh) in &all {
        fill(&mut img, x, y, rw, rh, fg);
    }

    (img.into_raw(), size, size)
}
