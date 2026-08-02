# Промт для GitHub Copilot: HyperX NGENUITY Open

## Контекст

Ты работаешь над форком проекта **HyperHeadset** (LennardKittner/HyperHeadset) — open-source утилиты для HyperX Cloud II Wireless на Rust. 

Оригинал: MIT-лицензия, стек Rust + hidapi + enigo + winit/ksni.

**Задача:** Перестроить проект в полноценное кроссплатформенное приложение с GUI (egui/eframe), системным эквалайзером, Discord-интеграцией и настройками гарнитуры.

---

## Архитектура проекта

```
hyperx-ngenuity-open/
├── Cargo.toml
├── src/
│   ├── main.rs              # Точка входа, tokio runtime, device thread
│   ├── lib.rs               # Экспорты, макросы, DeviceEvent enum
│   ├── config.rs            # TOML конфиг (serde): Discord, Audio, Device, Input
│   ├── gui/
│   │   ├── mod.rs           # eframe::App, табы, dashboard, reconnect handling
│   │   ├── eq_tab.rs        # 10-полосный EQ с debounce, пресеты, file dialog
│   │   └── discord_tab.rs   # Выбор режима: None/Keybind/Direct + hotkey capture
│   ├── device/
│   │   └── mod.rs           # HID протокол HyperX (0x21 0xBB prefix)
│   ├── audio/
│   │   ├── mod.rs           # AudioBackend trait + platform dispatch
│   │   ├── debounce.rs      # DebouncedEQ (500ms delay)
│   │   ├── windows.rs       # Equalizer APO integration
│   │   ├── linux.rs         # EasyEffects / PipeWire integration
│   │   └── macos_eqmac.rs   # eqMac HTTP API integration
│   ├── discord/
│   │   ├── mod.rs           # Discord IPC Rich Presence
│   │   └── rpc_ws.rs        # Discord local RPC WebSocket (bidirectional mute)
│   ├── input/
│   │   └── mod.rs           # Smart Mute Handler (4 modes)
│   ├── hotkey/
│   │   ├── mod.rs           # GlobalHotkeyCapture trait
│   │   ├── windows.rs       # Raw Input + GetAsyncKeyState
│   │   ├── linux.rs         # evdev implementation
│   │   └── macos.rs         # Carbon placeholder
│   ├── dialog/
│   │   └── mod.rs           # Cross-platform file dialogs + PresetFile format
│   └── tray/
│       ├── mod.rs           # TrayBackend trait
│       ├── windows.rs       # tray-icon implementation
│       ├── linux.rs         # ksni implementation
│       └── macos.rs         # tray-icon implementation
```

---

## Технические ограничения (КРИТИЧНО)

1. **HyperX Cloud II Wireless НЕ имеет HID-команд для EQ.** Эквалайзер в NGENUITY — чисто софтверный. EQ в нашем проекте — СИСТЕМНЫЙ, через ОС.
2. **HID-протокол:** 31-байтовые пакеты, prefix `[0x21, 0xBB]`. Команды:
   - Battery: `0x0b` → response byte[3] = percent
   - Mute status: `0x23` → response byte[3] = 0x00/0x01
   - Toggle mute: `0x24, 0x01`
   - Sidetone: `0x10, enabled`
   - Voice prompts: `0x12, enabled`
   - Auto-shutdown: `0x14, minutes`
3. **Vendor/Product IDs:** 0x03f0/0x018b (HP), 0x0951/0x018b (Kingston), 0x03f0/0x018c
4. **Discord IPC:** discord-rich-presence crate. Чтение mute-статуса — через локальный WebSocket RPC (ws://127.0.0.1:6463).

---

## Реализованные фичи (ВСЁ)

### ✅ Smart Mute Handler (`src/input/mod.rs`)
4 режима: Standard, MediaPlayPause, SmartDouble, SmartHold

### ✅ Debounced EQ (`src/audio/debounce.rs`)
500ms delay, background worker thread

### ✅ Windows EQ — Equalizer APO (`src/audio/windows.rs`)
config.txt generation, preset save/load/list

### ✅ Linux EQ — EasyEffects / PipeWire (`src/audio/linux.rs`)
JSON preset format, `easyeffects --load-preset`, `pw-cli` fallback

### ✅ macOS EQ — eqMac (`src/audio/macos_eqmac.rs`)
HTTP API localhost:8080, async via tokio block_on

### ✅ Discord интеграция (`src/discord/`)
- Keybind mode: enigo emulation
- Direct mode: Rich Presence + WebSocket RPC (bidirectional)
- None mode: disabled

### ✅ Global Hotkey Capture (`src/hotkey/`)
- **Windows**: Raw Input + GetAsyncKeyState (F13-F24, Media keys, modifiers)
- **Linux**: evdev /dev/input/event* (requires input group or sudo)
- **macOS**: placeholder (Carbon)

### ✅ File Dialogs (`src/dialog/mod.rs`)
- Import/Export presets (.hyperx JSON format)
- **Windows**: PowerShell + System.Windows.Forms
- **Linux**: zenity / kdialog
- **macOS**: osascript

### ✅ System Tray (`src/tray/`)
- **Windows**: tray-icon, colored battery icon
- **Linux**: ksni (D-Bus StatusNotifierItem)
- **macOS**: tray-icon with template icon

### ✅ Auto-reconnect GUI (`src/main.rs` + `src/gui/mod.rs`)
- DeviceEvent enum: StateChanged, Connected, Disconnected, BatteryLow
- Background thread sends events via channel
- GUI updates automatically on reconnect/disconnect
- Battery low warnings

---

## Код-стайл

- Rust 2021 edition
- `anyhow::Result` для ошибок
- `parking_lot::Mutex` вместо std::sync::Mutex
- `tokio` runtime для async
- Комментарии на русском или английском
- Модули разделять по файлам

---

## Пример ожидаемого результата

При запуске:
1. Открывается окно 900x600 с 5 табами
2. Dashboard: батарея, микрофон, сигнал, быстрые действия
3. EQ: 10 слайдеров с debounce, пресеты, импорт/экспорт
4. Input: 4 режима mute кнопки, тест
5. Discord: radio buttons, keybind запись, App ID, Rich Presence
6. Settings: sidetone, voice prompts, auto-shutdown, EQ статус
7. При отключении гарнитуры — GUI показывает "Нет подключения"
8. При переподключении — GUI автоматически обновляется
9. Закрытие окна → сворачивается в трей
10. Нажатие mute → срабатывает выбранный режим
