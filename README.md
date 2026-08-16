# HyperHeadsetv2 — Tauri Integration

## Структура
- `src/` — ваш текущий Rust код (библиотека + egui бинарник)
- `src-tauri/` — Tauri backend
- `ui/` — HTML/CSS/JS фронтенд

## Что изменилось
1. Добавлен `src/lib.rs` — экспорт модулей для Tauri
2. `Cargo.toml` — добавлены `[lib]` и `[[bin]]`
3. `src-tauri/` — новый Tauri проект, использует `hyperheadsetv2` как lib

## Сборка
```bash
# Tauri версия
cd src-tauri
cargo tauri build --release

# Старый egui (всё ещё работает)
cargo run --bin hyperheadsetv2-egui
```

## Tray Icon с батареей
Ваш `tray/icon.rs` работает без изменений.
В `src-tauri/src/main.rs` добавлено:
```rust
let (rgba, w, h) = generate_battery_icon_rgba(...);
let img = image::RgbaImage::from_raw(w, h, rgba).unwrap();
let mut png = Vec::new();
img.write_to(..., image::ImageFormat::Png).unwrap();
let tauri_img = tauri::image::Image::from_bytes(&png).unwrap();
tray.set_icon(Some(tauri_img));
```
