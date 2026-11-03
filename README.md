# hyperx-ngenuity-open

Unofficial open-source control app for **HyperX wireless headsets** (Windows, Linux, macOS).
A lightweight alternative to HyperX NGENUITY: battery, mute, sidetone, Discord integration,
automatic audio device switching and a readable battery icon in the tray.

> Unofficial project, not affiliated with, endorsed or sponsored by HP Inc. or HyperX.
> HyperX and NGENUITY are trademarks of HP Inc., used here only to describe compatible devices.

## Supported devices

HyperX Cloud II Wireless (incl. DTS), Cloud II Core Wireless, Cloud III Wireless, Cloud III S Wireless,
Cloud Alpha Wireless, Cloud Flight (S). More models can be added from HID logs (open an issue with the
VID:PID and the log).

## Features

- Battery level, charging state, tray icon with large readable digits; configurable colors, thresholds and a
  separate "headset off" icon.
- Mute button: standard / play-pause / smart modes (single, double, hold); sidetone button as play/pause.
- Discord: direct mute through local RPC (guided setup in the app), keybind mode or both; optional Discord -> headset sync.
- Auto audio switching: when the headset is switched off the system output/input move to the devices you chose
  (e.g. another headset) and come back when it is switched on again; running apps' streams are moved too.
- Voice announcements for battery events with adjustable pause, multi-headset failover, always-on log file.

## Install / build

Windows installer: run `./build_tauri_installer.sh` in Git Bash (needs Rust, Node and `cargo install tauri-cli --version "^2"`);
the installer and exe land in `release/`. The app title bar and the first log line show the version.
Config, tray settings and log live next to the exe.

```bash
cargo build --release          # egui build, bin `hyperx_ngenuity_open`
cd src-tauri && cargo tauri build   # Tauri app (recommended)
```

On Linux install `99-HyperHeadset.rules` into `/etc/udev/rules.d/` so the headset can be opened without root.

## Credits and license

Based on [HyperHeadset](https://github.com/LennardKittner/HyperHeadset) by Lennard Kittner (MIT), extended and
maintained by Sergey Ishkin. Released under the MIT license, see [LICENSE](LICENSE) and
[THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt). Protocol notes live in [REVERSE_ENGINEERING](REVERSE_ENGINEERING).

---

## По-русски

Неофициальное open-source приложение для управления беспроводными гарнитурами **HyperX**: батарея, мьют,
сайдтон, прямая интеграция с Discord, авто-переключение звука при выключении/включении гарнитуры и читаемая
иконка в трее. Проект не связан с HP Inc. и HyperX; HyperX и NGENUITY — торговые марки HP Inc.
Основан на HyperHeadset (Lennard Kittner, MIT).
