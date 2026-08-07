# Tray Icon Battery Percentage — Integration Guide

## What you get
- 16x16 tray icon with **battery percentage rendered as text**
- Color-coded background: 🟢 green (≥30%), 🔴 red (<30%), 🟡 yellow (charging)
- Icon cache — each unique (percent, charging) combo rendered once
- Tooltip with battery status

## Files to add

| File | Purpose |
|------|---------|
| `src/tray_battery_icon_state.rs` | Enum: NoDevice / Disconnected / Connected {percent, charging} |
| `src/tray_icon_renderer.rs` | 16x16 RGBA icon generator (pixel-art digits) |
| `src/tray_manager.rs` | TrayIcon wrapper with cache + update logic |
| `src/tray_config.rs` | Settings: colors, refresh interval, monochrome |

## Cargo.toml additions

```toml
[dependencies]
tray-icon = { version = "0.19", default-features = false, features = ["libxdo"] }
image = { version = "0.25", default-features = false, features = ["png"] }
```

## Integration into main.rs

### 1. Add modules
```rust
mod tray_battery_icon_state;
mod tray_icon_renderer;
mod tray_manager;
mod tray_config;
```

### 2. Create TrayManager before eframe
```rust
fn main() {
    let mut tray_manager = tray_manager::TrayManager::new();

    // ... your existing device thread ...

    // In the device thread loop, after refresh_state():
    tray_manager.update(Some(device.get_state()));

    // Or pass via channel from device thread to main thread
}
```

### 3. If using eframe + tray-icon together
Tray-icon needs a winit event loop. Options:

**Option A: Tray in separate thread (recommended)**
```rust
use std::sync::{Arc, Mutex};

let tray_state = Arc::new(Mutex::new(None::<DeviceState>));
let tray_state_clone = Arc::clone(&tray_state);

std::thread::spawn(move || {
    let mut tray = TrayManager::new();
    loop {
        if let Ok(state) = tray_state_clone.lock() {
            tray.update(state.as_ref());
        }
        std::thread::sleep(Duration::from_secs(3));
    }
});

// In device thread:
*tray_state.lock().unwrap() = Some(device.get_state().clone());
```

**Option B: Tray via eframe custom event loop**
```rust
// In your eframe App::update():
if let Some(tray) = &mut self.tray_manager {
    tray.update(Some(&self.device_state));
}
```

## Customization

### Change colors
Edit `tray_config.rs`:
```rust
color_high: [0, 150, 255],     // Blue instead of green
color_low: [255, 0, 0],         // Bright red
```

### Disable percentage (show static icon)
```rust
tray_manager.update(None);  // Shows default headset icon
```

### Add right-click menu
In `tray_manager.rs`, modify `TrayIconBuilder`:
```rust
let menu = Menu::new();
let quit = MenuItem::new("Quit", true, None);
menu.append(&quit).unwrap();

tray_icon = TrayIconBuilder::new()
    .with_menu(Box::new(menu))
    .with_icon(create_default_tray_icon())
    .build()
    .ok();
```

## How it works (LennardKittner approach)

1. **State enum** maps DeviceState → TrayBatteryIconState
2. **WindowsIconKey** = (percent, charging) — unique cache key
3. **render_windows_battery_icon_rgba()** draws:
   - Background rectangle (color based on percent/charging)
   - Pixel-art digits (2x2 or 1x1 scale, auto-fitted to 16x16)
   - Special compact layout for "100"
4. **HashMap cache** stores raw RGBA bytes — no re-rendering
5. **tray-icon crate** sets the icon via OS API

## Troubleshooting

| Problem | Fix |
|---------|-----|
| Icon not showing | Ensure `tray-icon` feature flags match your OS |
| Blurry text | Windows tray is 16x16 — text is pixel-art by design |
| "100" doesn't fit | Special case handled — uses 3-digit compact layout |
| Memory leak | Icon cache grows to max 101 entries (0-100% + charging) — negligible |

## Assets needed

Place a `headphone.png` (16x16 or 32x32) in `assets/` for the default icon:
```
assets/
└── headphone.png
```

Or replace `create_default_tray_icon()` with an embedded icon:
```rust
let bytes = include_bytes!("../assets/headphone.png");
```
