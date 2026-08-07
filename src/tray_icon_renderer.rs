use image::{Rgba, RgbaImage};
use crate::tray_battery_icon_state::WindowsIconKey;

const WINDOWS_ICON_SIZE: u32 = 16;

fn draw_rect(image: &mut RgbaImage, x: i32, y: i32, width: i32, height: i32, color: Rgba<u8>) {
    for px in x.max(0)..(x + width).min(WINDOWS_ICON_SIZE as i32) {
        for py in y.max(0)..(y + height).min(WINDOWS_ICON_SIZE as i32) {
            image.put_pixel(px as u32, py as u32, color);
        }
    }
}

fn draw_digit(image: &mut RgbaImage, digit: char, x: i32, y: i32, scale: i32, color: Rgba<u8>) {
    let rows = match digit {
        '0' => &["111", "101", "101", "101", "111"],
        '1' => &["01", "01", "01", "01", "01"],
        '2' => &["111", "001", "111", "100", "111"],
        '3' => &["111", "001", "111", "001", "111"],
        '4' => &["101", "101", "111", "001", "001"],
        '5' => &["111", "100", "111", "001", "111"],
        '6' => &["111", "100", "111", "101", "111"],
        '7' => &["111", "001", "010", "010", "010"],
        '8' => &["111", "101", "111", "101", "111"],
        '9' => &["111", "101", "111", "001", "111"],
        _ => &["000", "000", "000", "000", "000"],
    };

    for (row_index, row) in rows.iter().enumerate() {
        for (col_index, bit) in row.chars().enumerate() {
            if bit == '1' {
                draw_rect(
                    image,
                    x + (col_index as i32 * scale),
                    y + (row_index as i32 * scale),
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

/// Render a 16x16 RGBA battery percentage icon for Windows tray.
/// 
/// Color coding:
/// - Charging: yellow background
/// - < 30%: red background  
/// - >= 30%: green background
/// - Text: dark gray
pub fn render_windows_battery_icon_rgba(key: WindowsIconKey) -> Vec<u8> {
    let mut image = RgbaImage::from_pixel(
        WINDOWS_ICON_SIZE,
        WINDOWS_ICON_SIZE,
        Rgba([0, 0, 0, 0]),
    );

    let background_color = if key.charging {
        Rgba([245, 216, 64, 255])   // Yellow
    } else if key.percent < 30 {
        Rgba([220, 90, 90, 255])    // Red
    } else {
        Rgba([96, 196, 106, 255])   // Green
    };

    draw_rect(
        &mut image,
        0,
        0,
        WINDOWS_ICON_SIZE as i32,
        WINDOWS_ICON_SIZE as i32,
        background_color,
    );

    // Special layout for "100" — compact 3-digit on 16x16
    if key.percent == 100 {
        let text_color = Rgba([10, 10, 10, 255]);
        let y = 3;

        // "1" (narrow)
        draw_rect(&mut image, 1, y, 1, 10, text_color);
        draw_rect(&mut image, 0, y + 9, 3, 1, text_color);

        // First "0"
        let z1 = 4;
        draw_rect(&mut image, z1, y, 5, 1, text_color);
        draw_rect(&mut image, z1, y + 9, 5, 1, text_color);
        draw_rect(&mut image, z1, y, 1, 10, text_color);
        draw_rect(&mut image, z1 + 4, y, 1, 10, text_color);

        // Second "0"
        let z2 = 10;
        draw_rect(&mut image, z2, y, 5, 1, text_color);
        draw_rect(&mut image, z2, y + 9, 5, 1, text_color);
        draw_rect(&mut image, z2, y, 1, 10, text_color);
        draw_rect(&mut image, z2 + 4, y, 1, 10, text_color);

        return image.into_raw();
    }

    let text = key.percent.to_string();
    let mut scale = 2;
    let spacing = if text.len() >= 3 { 0 } else { 1 };
    let horizontal_padding = if text.len() >= 3 { 0 } else { 1 };
    let inner_left = horizontal_padding;
    let inner_right = (WINDOWS_ICON_SIZE as i32 - 1 - horizontal_padding).max(inner_left);
    let usable_width = (inner_right - inner_left + 1).max(1);

    let mut glyph_widths: Vec<i32> = text
        .chars()
        .map(|digit| if digit == '1' { 2 * scale } else { 3 * scale })
        .collect();
    let mut total_width: i32 = glyph_widths.iter().sum::<i32>()
        + spacing * (text.chars().count().saturating_sub(1) as i32);

    if total_width > usable_width && scale > 1 {
        scale = 1;
        glyph_widths = text
            .chars()
            .map(|digit| if digit == '1' { 2 * scale } else { 3 * scale })
            .collect();
        total_width = glyph_widths.iter().sum::<i32>()
            + spacing * (text.chars().count().saturating_sub(1) as i32);
    }

    let centered_start_x = inner_left + ((usable_width - total_width).max(0) / 2);
    let max_start_x = (inner_right - total_width + 1).max(inner_left);
    let start_x = centered_start_x.clamp(inner_left, max_start_x);
    let start_y = if scale == 2 { 3 } else { 5 };

    let mut x = start_x;
    let text_color = Rgba([10, 10, 10, 255]);
    for (idx, digit) in text.chars().enumerate() {
        draw_digit(&mut image, digit, x, start_y, scale, text_color);
        x += glyph_widths[idx] + spacing;
    }

    image.into_raw()
}

/// Default headset icon when no device connected
pub fn create_default_tray_icon() -> tray_icon::Icon {
    let bytes = include_bytes!("../assets/headphone.png");
    let img = image::load_from_memory(bytes).unwrap().into_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).unwrap()
}
