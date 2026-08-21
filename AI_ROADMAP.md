# HyperHeadset_v2 — Roadmap for AI Agents

## Project goal

HyperHeadset_v2 is a cross-platform Tauri 2 desktop application for monitoring and controlling compatible HyperX headsets through the existing Rust core/HID implementation.

Target platforms:

- Windows
- Linux
- macOS

The project already contains a working/partially working Rust core and a Tauri UI. The current task is to finish the integration without replacing working device logic unnecessarily.

## Current branch

Work is being developed on:

`fix/tauri-functional-hid`

Do not merge into `main` until Windows, Linux and macOS builds have been checked.

## Current state

### Tauri integration

- Tauri 2 UI exists under `ui/`.
- `src-tauri/src/main.rs` exposes device commands through Tauri commands.
- `src-tauri/tauri.conf.json` already has `withGlobalTauri: true`.
- Main window and tray integration exist.
- The main window close action is intended to hide the window instead of terminating the application.
- Tray left click is intended to restore/focus the main window.

### Device integration

The Tauri backend currently uses:

- `HyperXDevice`
- `DeviceState`
- `DeviceCommand`
- `refresh_state()`
- `connect()` / `disconnect()`

The UI should communicate with the Rust device layer only through stable Tauri commands/events. Do not duplicate HID logic in JavaScript.

### Disconnect detection

A heartbeat-based disconnect mechanism has been introduced. HID refresh failures are counted before resetting the device handle. The code also attempts to distinguish HID communication failure from Windows device enumeration state.

Important: the current logging text mentions Windows enumeration. This must be made platform-aware before shipping Linux/macOS builds. Do not call Windows-only APIs from non-Windows targets.

## Immediate priorities

### P0 — Build correctness

1. Keep `src/dialog/mod.rs` and `src/hotkey/mod.rs` as the canonical module implementations.
2. Do not reintroduce duplicate `src/dialog.rs` / `src/hotkey.rs` modules.
3. Fix all compilation errors on Windows.
4. Remove or gate unused platform-specific imports where appropriate.
5. Verify `cargo tauri build` succeeds.

### P0 — Tray behaviour

Verify all of the following:

- Tray icon is created on startup.
- Left click opens/restores the main window.
- `Открыть` opens/restores the main window.
- Closing the main window hides it instead of exiting.
- `Выход` actually terminates the application.
- Compact window can be opened from the tray.
- Tray tooltip updates with battery state.
- Disconnected state restores the default tray icon and removes stale battery percentage.

Do not implement tray behaviour with OS-specific code unless required by the platform API.

### P0 — Tauri device commands

Verify the full path for:

- Get device state
- Toggle mute
- Set sidetone
- Set voice prompts
- Compact window

Expected architecture:

`UI -> Tauri invoke -> Rust command -> DeviceCommand -> HID/device core`

and:

`device thread -> Tauri event -> UI`

Do not put direct HID access in `ui/app.js`.

### P0 — HID disconnect diagnosis

The application must distinguish these cases where possible:

1. Physical USB/HID device disappeared.
2. Device is still enumerated but the current HID handle stopped responding.
3. A single command failed but the device is still healthy.
4. Temporary HID timeout.

Do not mark the headset disconnected because one optional command failed.

Recommended state machine:

`Connected -> transient failure -> retry -> handle reset -> reconnect -> Disconnected`

Use bounded retries and exponential/backoff delays where appropriate. Avoid tight loops.

### P1 — Cross-platform abstraction

Audit every platform-specific section.

Use Rust conditional compilation:

```rust
#[cfg(target_os = "windows")]
```

```rust
#[cfg(target_os = "linux")]
```

```rust
#[cfg(target_os = "macos")]
```

Windows-specific functionality must not leak into Linux/macOS compilation.

Examples requiring review:

- HID enumeration
- audio control
- global hotkeys
- file dialogs
- system tray behaviour
- Windows command execution / PowerShell
- device paths

If a feature cannot be implemented on a platform, expose a clean capability/unsupported result instead of breaking the build.

### P1 — HID implementation

Inspect the existing `HyperXDevice` implementation before changing protocol code.

Do not guess or redesign HID packets unless logs or protocol evidence require it.

Document:

- VID/PID
- HID usage page/usage
- interface/path selection
- report IDs
- battery request/response
- mute request/response
- sidetone request/response
- voice prompt request/response
- charging detection

Add diagnostic logging behind a debug/logging mechanism rather than unconditional noisy production logging.

### P1 — Device lifecycle

The device thread should have a single owner of the HID handle.

Commands from UI/tray should be sent through `DeviceCommand`.

Avoid opening the same HID device from multiple threads.

On disconnect:

- close/reset the handle;
- clear stale state;
- emit `device-disconnected`;
- emit a default `device-state`;
- reset tray battery icon/tooltip;
- continue scanning for reconnection.

On reconnect:

- refresh complete state;
- update shared state;
- emit `device-connected`;
- emit `device-state`;
- update tray.

## P2 — UI correctness

The UI must never display the last battery percentage indefinitely after a real disconnect.

When disconnected:

- `connected = false`
- battery value should be treated as unavailable/default
- charging should be false/default
- mute/sidetone state should not be presented as authoritative
- UI should clearly show no device

When connected, UI should update from `device-state` events.

Avoid polling from JavaScript if the Rust device thread already provides state events.

## P2 — Testing matrix

### Windows

- USB headset/dongle connected at startup
- unplug USB
- plug USB back in
- headset powered off/on if detectable separately
- temporary HID timeout
- NGENUITY running
- NGENUITY closed
- tray click
- window close
- application exit
- mute
- sidetone
- voice prompts
- battery and charging

### Linux

- build
- launch
- tray
- HID enumeration/permissions
- device connect/disconnect
- unsupported features handled gracefully

Document required udev rules if needed.

### macOS

- build
- launch
- tray/menu bar
- HID permissions/access
- device connect/disconnect
- unsupported features handled gracefully

Document required entitlements/permissions if needed.

## P2 — Logging

Use consistent prefixes:

- `[App]`
- `[Tauri]`
- `[Tray]`
- `[Device]`
- `[HID]`
- `[Audio]`
- `[Hotkey]`

For disconnect diagnosis, record enough information to answer:

- Was the device enumerated?
- Was the HID handle open?
- Which command failed?
- How many consecutive failures occurred?
- Was a reconnect attempted?

Never log sensitive user information.

## P3 — Code quality

- Remove unused imports.
- Avoid `unwrap()` in long-running device/tray code where an error can be handled safely.
- Avoid panics in the device monitoring thread.
- Keep platform-specific code isolated.
- Keep Tauri command functions small.
- Keep protocol/HID logic in the Rust core rather than `main.rs`.
- Add tests for state transitions where practical.

## P3 — Documentation

README should eventually contain:

- supported platforms
- build prerequisites
- development build commands
- release build commands
- HID permissions/setup
- known limitations
- troubleshooting
- disconnect diagnostics

## Build commands

From repository root:

```bash
cargo tauri dev
```

Release:

```bash
cargo tauri build
```

## Rules for future AI agents

1. Read this file before modifying the project.
2. Inspect existing code before inventing new abstractions.
3. Preserve working Rust core functionality.
4. Do not replace the Tauri UI with another framework.
5. Do not add Windows-only code without `cfg` guards.
6. Do not assume a HID communication failure means physical USB removal.
7. Do not delete protocol/device code merely because it is unfamiliar.
8. Keep changes small and build after each logical change.
9. Prefer a separate branch for substantial changes.
10. Record important architectural decisions in this roadmap.
11. Test Windows first for the HyperX hardware path, then ensure Linux/macOS still compile.
12. Before merging, verify tray, Tauri commands, HID state, disconnect/reconnect and clean application exit.

## Definition of done

The project is considered complete when:

- `cargo tauri build` succeeds on Windows, Linux and macOS.
- The application starts without a headset connected.
- A headset can be detected after application startup.
- Battery state is shown correctly.
- Disconnect does not leave a stale battery percentage.
- Reconnect restores the device state automatically.
- Mute/sidetone/voice prompt controls work through Tauri.
- Tray opens/restores the main window.
- Closing the main window hides it to tray.
- Explicit Exit terminates the application.
- Platform-specific code is correctly gated.
- No known regression is introduced into the existing Rust core.
