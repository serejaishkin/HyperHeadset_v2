use image::{RgbaImage, Rgba};

use super::config::TrayIconConfig;

const DIGITS: [[u8; 7]; 10] = [
    [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
    [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
    [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
    [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
    [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
    [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
    [0b01110, 0b10001, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
    [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
    [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
    [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b10001, 0b01110],
];

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

    let text = format!("{}", percent);
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len() as u32;
    let gap = 2;
    let max_digit_w = (size - gap * n.saturating_sub(1)) / (5 * n);
    let max_digit_h = size / 7;
    let scale = max_digit_w.min(max_digit_h).max(1);
    let digit_w = 5 * scale;
    let digit_h = 7 * scale;
    let total_w = n * digit_w + n.saturating_sub(1) * gap;
    let start_x = size.saturating_sub(total_w) / 2;
    let start_y = size.saturating_sub(digit_h) / 2;
    let outline_px = (scale as i32 / 3).max(1);

    for (idx, ch) in chars.iter().enumerate() {
        let d = ch.to_digit(10).unwrap_or(0) as usize;
        let digit = DIGITS[d];
        let off_x = start_x + idx as u32 * (digit_w + gap);
        for row in 0..7u32 {
            for col in 0..5u32 {
                if (digit[row as usize] >> (4 - col)) & 1 == 1 {
                    for dy in -outline_px..=outline_px {
                        for dx in -outline_px..=outline_px {
                            let xi = off_x as i32 + col as i32 * scale as i32 + dx;
                            let yi = start_y as i32 + row as i32 * scale as i32 + dy;
                            if xi >= 0 && yi >= 0 {
                                let x = xi as u32; let y = yi as u32;
                                if x < size && y < size { img.put_pixel(x, y, fg); }
                            }
                        }
                    }
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = off_x + col * scale + dx;
                            let y = start_y + row * scale + dy;
                            if x < size && y < size { img.put_pixel(x, y, fg); }
                        }
                    }
                }
            }
        }
    }

    (img.into_raw(), size, size)
}
