# HyperHeadset v2 — AI Roadmap

## Goal
Keep the existing Rust/HID functionality and reproduce the stable legacy UI in the Tauri 2 frontend. Do not replace the working headset backend with browser-only logic.

## Current priority
1. Use the fixed legacy interface as the visual/source-of-truth reference.
2. Rebuild that interface in `ui/index.html`, `ui/styles.css`, `ui/app.js`.
3. Wire every visible control to Tauri commands/events.
4. Make tray open/hide/exit reliable.
5. Implement robust HID heartbeat/reconnect and clear stale battery state.
6. Verify Windows, Linux and macOS builds.

## Architecture
Rust core = HID/device/audio/hotkeys/dialogs. Tauri = commands/events/window/tray. UI = HTML/CSS/JS only.

## Cross-platform rules
Use `#[cfg(target_os = "windows")]`, `linux`, and `macos` for OS-specific code. Do not put unconditional Windows APIs in common modules. Frontend must not depend on OS-specific implementation details.

## HID/disconnect
Battery polling is the primary heartbeat. Transient command failures must not immediately mean USB removal. Where supported, distinguish HID I/O failure from device enumeration failure. Confirmed disconnect clears stale battery/mute/charging state and emits an event. Reconnect automatically when the device returns.

## UI mapping
Dashboard: battery, microphone status, signal, quick actions. Left controls: volume, microphone, mic mute, sidetone, playback/voice action. Settings: headset and voice notification settings. Header: connection state and battery indicator.

## Definition of Done
Tauri builds on Windows/Linux/macOS; legacy UI behavior is reproduced; visible controls are functional or explicitly unavailable; tray reliably shows/hides/exits; disconnect/reconnect never leaves stale battery percentage; history is not force-rewritten.
