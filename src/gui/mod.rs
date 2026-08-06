use eframe::egui;
use crate::config::{Config, MuteButtonMode};
use crate::tray::icon::TrayIconConfig;
use crate::device::DeviceState;
use crate::audio::debounce::DebouncedEQ;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub mod eq_tab;
pub mod discord_tab;

pub struct HyperXApp {
    pub config: Config,
    pub device_state: DeviceState,
    pub discord_connected: bool,
    pub selected_tab: Tab,
    pub eq_bands: [f32; 10],
    pub needs_save: bool,
    pub apo_available: bool,
    pub tray_tx: Option<std::sync::mpsc::Sender<crate::tray::TrayCommand>>,
    pub tray: Option<crate::tray::PlatformTray>,
    pub debounced_eq: Arc<DebouncedEQ>,
    pub recording_keybind: bool,
    pub device_rx: Option<std::sync::mpsc::Receiver<crate::DeviceEvent>>,
    pub device_cmd_tx: Option<std::sync::mpsc::Sender<crate::DeviceCommand>>,
    pub keybind_capture: Option<Arc<crate::hotkey::GlobalHotkeyCapture>>,
    pub last_battery_warning: Option<u8>,
    pub tray_rx: Option<std::sync::mpsc::Receiver<crate::tray::TrayCommand>>,
    pub volume: f32,
    pub mic_volume: f32,
    pub should_exit: bool,
    pub window_hidden: bool,
    pub last_volume_check: Instant,
    pub show_discord_panel: bool,
    pub selected_settings_tab: SettingsTab,
    pub tray_icon_config: TrayIconConfig,
    #[cfg(target_os = "windows")]
    pub volume_controller: Option<crate::platform::windows::volume::WindowsVolume>,
    #[cfg(target_os = "linux")]
    pub volume_controller: Option<crate::platform::linux::volume::LinuxVolume>,
    #[cfg(target_os = "macos")]
    pub volume_controller: Option<crate::platform::macos::volume::MacOSVolume>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsTab {
    Headset,
    Voice,
    TrayIcon,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Dashboard,
    Equalizer,
    Settings,
}

impl Default for Tab {
    fn default() -> Self { Tab::Dashboard }
}

impl HyperXApp {
    pub fn new(config: Config, device_state: DeviceState, apo_available: bool, debounced_eq: Arc<DebouncedEQ>) -> Self {
        let eq_bands = config.audio.eq_bands;
        Self {
            config,
            device_state,
            discord_connected: false,
            selected_tab: Tab::Dashboard,
            eq_bands,
            needs_save: false,
            apo_available,
            tray_tx: None,
            tray: None,
            debounced_eq,
            recording_keybind: false,
            device_rx: None,
            device_cmd_tx: None,
            keybind_capture: None,
            last_battery_warning: None,
            tray_rx: None,
            volume: 50.0,
            mic_volume: 50.0,
            should_exit: false,
            window_hidden: false,
            last_volume_check: Instant::now(),
            show_discord_panel: false,
            selected_settings_tab: SettingsTab::Headset,
            tray_icon_config: TrayIconConfig::load_or_create(),
            #[cfg(target_os = "windows")]
            volume_controller: Some(crate::platform::windows::volume::WindowsVolume::new()),
            #[cfg(target_os = "linux")]
            volume_controller: Some(crate::platform::linux::volume::LinuxVolume::new()),
            #[cfg(target_os = "macos")]
            volume_controller: Some(crate::platform::macos::volume::MacOSVolume::new()),
        }
    }

    pub fn with_tray(mut self, tx: std::sync::mpsc::Sender<crate::tray::TrayCommand>) -> Self {
        self.tray_tx = Some(tx);
        self
    }

    pub fn with_tray_backend(mut self, tray: crate::tray::PlatformTray) -> Self {
        self.tray = Some(tray);
        self
    }

    pub fn with_device_receiver(mut self, rx: std::sync::mpsc::Receiver<crate::DeviceEvent>) -> Self {
        self.device_rx = Some(rx);
        self
    }

    pub fn with_device_command_sender(mut self, tx: std::sync::mpsc::Sender<crate::DeviceCommand>) -> Self {
        self.device_cmd_tx = Some(tx);
        self
    }

    pub fn with_tray_receiver(mut self, rx: std::sync::mpsc::Receiver<crate::tray::TrayCommand>) -> Self {
        self.tray_rx = Some(rx);
        self
    }

    fn save_config(&mut self) {
        self.config.audio.eq_bands = self.eq_bands;
        log::info!("[GUI] Saving config...");
        if let Err(e) = self.config.save() {
            log::error!("[GUI] Failed to save config: {}", e);
        } else {
            log::info!("[GUI] Config saved successfully");
            self.needs_save = false;
        }
    }

    fn apply_eq_preset(&mut self, preset: &str) {
        log::info!("[GUI] EQ preset applied: {}", preset);
        self.eq_bands = match preset {
            "Flat" => [0.0; 10],
            "Bass Boost" => [6.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "Bass Cut" => [-6.0, -4.0, -2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "Treble Boost" => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 4.0, 6.0, 8.0],
            "Voice Chat" => [-2.0, 0.0, 2.0, 4.0, 6.0, 6.0, 4.0, 2.0, 0.0, -2.0],
            "Gaming" => [4.0, 3.0, 2.0, 1.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0],
            _ => self.eq_bands,
        };
        self.needs_save = true;
    }
}

impl eframe::App for HyperXApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(100));

        // HyperX dark theme
        ctx.set_visuals(egui::Visuals::dark());
        ctx.style_mut(|s| {
            s.visuals.selection.bg_fill = egui::Color32::from_rgb(200, 30, 30);
            s.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(200, 30, 30);
            s.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(160, 20, 20);
        });

        if let Some(tray) = &mut self.tray {
            tray.poll();
        }

        // Tray commands
        if let Some(rx) = &self.tray_rx {
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    crate::tray::TrayCommand::ToggleMute => {
                        if let Some(tx) = &self.device_cmd_tx {
                            let _ = tx.send(crate::DeviceCommand::ToggleMute);
                        }
                    }
                    crate::tray::TrayCommand::ShowWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        self.window_hidden = false;
                    }
                    crate::tray::TrayCommand::Quit => {
                        std::process::exit(0);
                    }
                    crate::tray::TrayCommand::RefreshBattery => {}
                }
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            std::process::exit(0);
        }

        if !self.window_hidden {
            if ctx.input(|i| i.viewport().minimized) == Some(true) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                self.window_hidden = true;
            }
        }

        if let Some(capture) = &self.keybind_capture {
            if let Some(combo) = capture.poll_result() {
                let key_str = combo.display.clone();
                self.config.discord.keybind = Some(key_str.clone());
                crate::input::GLOBAL_MUTE_HANDLER.set_keybind(Some(key_str));
                self.recording_keybind = false;
                self.keybind_capture = None;
                self.needs_save = true;
            }
        }

        if let Some(keybind) = &self.config.discord.keybind {
            crate::input::GLOBAL_MUTE_HANDLER.set_keybind(Some(keybind.clone()));
        }

        // Device events
        if let Some(rx) = &self.device_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    crate::DeviceEvent::StateChanged(state) => {
                        self.device_state = state.clone();
                        if let Some(tray) = &mut self.tray {
                            tray.update_battery(state.battery_percent, state.charging);
                            tray.update_mute(state.muted);
                        }
                    }
                    crate::DeviceEvent::Connected => {
                        self.device_state.connected = true;
                    }
                    crate::DeviceEvent::Disconnected => {
                        self.device_state.connected = false;
                    }
                    crate::DeviceEvent::BatteryLow(percent) => {
                        if self.last_battery_warning != Some(percent) {
                            self.last_battery_warning = Some(percent);
                        }
                    }
                }
            }
        }

        // Volume polling (master + mic) - all platforms
        if self.last_volume_check.elapsed() > Duration::from_millis(200) {
            if let Some(ref controller) = self.volume_controller {
                if let Some(vol) = controller.get_master_volume() {
                    self.volume = vol;
                }
                if let Some(mic_vol) = controller.get_microphone_volume() {
                    self.mic_volume = mic_vol;
                }
            }
            self.last_volume_check = Instant::now();
        }

        // === LEFT PANEL: sound + microphone (always visible) ===
        egui::SidePanel::left("audio_panel")
            .resizable(false)
            .default_width(100.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);

                    // --- Master Volume ---
                    ui.heading("VOL");
                    let mut vol = self.volume;
                    ui.add(
                        egui::Slider::new(&mut vol, 0.0..=100.0)
                            .vertical()
                            .show_value(false)
                            .text("")
                    );
                    if vol != self.volume {
                        self.volume = vol;
                        if let Some(ref controller) = self.volume_controller {
                            controller.set_master_volume(vol);
                        }
                    }
                    ui.label(format!("{:.0}%", self.volume));

                    ui.add_space(15.0);

                    // --- Mic Volume ---
                    ui.heading("MIC");
                    let mut mic_vol = self.mic_volume;
                    ui.add(
                        egui::Slider::new(&mut mic_vol, 0.0..=100.0)
                            .vertical()
                            .show_value(false)
                            .text("")
                    );
                    if mic_vol != self.mic_volume {
                        self.mic_volume = mic_vol;
                        if let Some(ref controller) = self.volume_controller {
                            controller.set_microphone_volume(mic_vol);
                        }
                    }
                    ui.label(format!("{:.0}%", self.mic_volume));

                    ui.add_space(10.0);

                    // --- Mic Mute ---
                    let is_mic_muted = self.volume_controller.as_ref()
                        .and_then(|c| c.get_microphone_mute())
                        .unwrap_or(self.device_state.muted);
                    let mic_label = if is_mic_muted { "MUTE" } else { "MIC ON" };
                    if ui.button(mic_label).clicked() {
                        if let Some(ref controller) = self.volume_controller {
                            controller.set_microphone_mute(!is_mic_muted);
                        } else if let Some(tx) = &self.device_cmd_tx {
                            let _ = tx.send(crate::DeviceCommand::ToggleMute);
                        }
                    }

                    ui.add_space(10.0);

                    // --- Sidetone ---
                    let mut sidetone = self.device_state.sidetone;
                    if ui.checkbox(&mut sidetone, "Sidetone").changed() {
                        if let Some(tx) = &self.device_cmd_tx {
                            let _ = tx.send(crate::DeviceCommand::SetSidetone(sidetone));
                        }
                        self.device_state.sidetone = sidetone;
                    }

                    ui.add_space(15.0);

                    // --- Media Play/Pause ---
                    if ui.add_sized([50.0, 40.0], egui::Button::new("PLAY")).clicked() {
                        crate::input::GLOBAL_MUTE_HANDLER.do_media_play_pause();
                    }
                });
            });

        // === TOP PANEL ===
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("HyperX NGENUITY Open");
                ui.separator();

                ui.selectable_value(&mut self.selected_tab, Tab::Dashboard, "Dashboard");
                ui.selectable_value(&mut self.selected_tab, Tab::Equalizer, "Equalizer");
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "Settings");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Discord").clicked() {
                        self.show_discord_panel = !self.show_discord_panel;
                    }
                    ui.separator();
                    if self.device_state.connected {
                        let icon = if self.device_state.charging { "[CHG]" } else { "[BAT]" };
                        ui.label(format!("{} {}%", icon, self.device_state.battery_percent));
                        ui.colored_label(egui::Color32::GREEN, "ON");
                    } else {
                        ui.colored_label(egui::Color32::RED, "OFF - No connection");
                    }
                });
            });
        });

        // === DISCORD PANEL (right) ===
        if self.show_discord_panel {
            egui::SidePanel::right("discord_panel")
                .resizable(true)
                .default_width(280.0)
                .max_width(350.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Discord");
                        if ui.button("X").clicked() {
                            self.show_discord_panel = false;
                        }
                    });
                    ui.separator();
                    discord_tab::show(
                        ui,
                        &mut self.config.discord,
                        self.discord_connected,
                        &mut self.needs_save,
                        &mut self.recording_keybind,
                        &mut self.keybind_capture,
                    );
                });
        }

        // === CENTER ===
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Dashboard => self.show_dashboard(ui),
                Tab::Equalizer => self.show_equalizer(ui),
                Tab::Settings => self.show_settings(ui),
            }
        });

        if self.needs_save {
            egui::TopBottomPanel::bottom("save_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "(!) Unsaved changes");
                    if ui.button("Save").clicked() { self.save_config(); }
                });
            });
        }
    }
}

impl HyperXApp {
    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Headset Status");
        ui.separator();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Battery:");
                if self.device_state.charging {
                    ui.colored_label(egui::Color32::YELLOW, "Charging");
                }
                ui.label(format!("{}%", self.device_state.battery_percent));
                let battery = self.device_state.battery_percent as f32 / 100.0;
                let color = if self.device_state.battery_percent > 30 { egui::Color32::GREEN }
                    else if self.device_state.battery_percent > 15 { egui::Color32::YELLOW }
                    else { egui::Color32::RED };
                ui.add(egui::ProgressBar::new(battery).fill(color).desired_width(200.0));
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.label("Microphone:");
                if self.device_state.muted {
                    ui.colored_label(egui::Color32::RED, "Muted");
                } else {
                    ui.colored_label(egui::Color32::GREEN, "Active");
                }
                ui.add_space(10.0);
                ui.label("Signal:");
                ui.label(format!("{} dBm", self.device_state.signal_dbm));
            });
        });

        ui.separator();
        ui.heading("Quick Actions");
        ui.horizontal(|ui| {
            if ui.button(if self.device_state.muted { "Unmute Mic" } else { "Mute Mic" }).clicked() {
                log::info!("[GUI] Dashboard: ToggleMute clicked");
                if let Some(tx) = &self.device_cmd_tx { let _ = tx.send(crate::DeviceCommand::ToggleMute); }
            }
            if ui.button("Check Battery (Voice)").clicked() {
                log::info!("[GUI] Dashboard: Check Battery voice clicked");
                crate::audio::voice::play(crate::audio::voice::VoiceEvent::Battery(self.device_state.battery_percent));
            }
        });
    }

    fn show_equalizer(&mut self, ui: &mut egui::Ui) {
        ui.heading("Equalizer");
        ui.separator();

        ui.horizontal_wrapped(|ui| {
            ui.label("Presets:");
            for preset in ["Flat", "Bass Boost", "Bass Cut", "Treble Boost", "Voice Chat", "Gaming"] {
                if ui.button(preset).clicked() {
                    self.apply_eq_preset(preset);
                }
            }
        });
        ui.separator();

        eq_tab::show(ui, &mut self.eq_bands, &mut self.needs_save, self.apo_available, &self.debounced_eq);
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.selected_settings_tab, SettingsTab::Headset, "Headset");
            ui.selectable_value(&mut self.selected_settings_tab, SettingsTab::Voice, "Voice");
            ui.selectable_value(&mut self.selected_settings_tab, SettingsTab::TrayIcon, "Tray Icon");
            ui.selectable_value(&mut self.selected_settings_tab, SettingsTab::Debug, "Debug");
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.selected_settings_tab {
                SettingsTab::Headset => self.show_settings_headset(ui),
                SettingsTab::Voice => self.show_settings_voice(ui),
                SettingsTab::TrayIcon => self.show_settings_tray_icon(ui),
                SettingsTab::Debug => self.show_settings_debug(ui),
            }
        });
    }

    fn show_settings_headset(&mut self, ui: &mut egui::Ui) {
        use crate::input::GLOBAL_MUTE_HANDLER;

        ui.heading("Headset Settings");
        ui.separator();

        ui.checkbox(&mut self.config.device.sidetone, "Sidetone (hear yourself)");
        if ui.checkbox(&mut self.config.device.voice_prompts, "Voice prompts").changed() {
            self.needs_save = true;
        }
        ui.horizontal(|ui| {
            ui.label("Auto-shutdown:");
            ui.add(egui::Slider::new(&mut self.config.device.auto_shutdown_minutes, 0..=60).text("min"));
            if ui.button("Apply").clicked() { self.needs_save = true; }
        });

        ui.separator();
        ui.heading("Mute Button Settings");
        ui.label("Headset mute button mode:");
        let prev_mode = self.config.input.mute_button_mode;
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.config.input.mute_button_mode, MuteButtonMode::Standard, "Standard\n(always MicMute)");
                ui.selectable_value(&mut self.config.input.mute_button_mode, MuteButtonMode::MediaPlayPause, "Always Play/Pause");
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.config.input.mute_button_mode, MuteButtonMode::SmartDouble, "Smart: single = mute\ndouble = Play/Pause");
                ui.selectable_value(&mut self.config.input.mute_button_mode, MuteButtonMode::SmartHold, "Smart: short = Play/Pause\nhold = mute");
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.config.input.mute_button_mode, MuteButtonMode::HoldPlayPause, "Smart: short = mute\nhold = Play/Pause");
            });
        });
        if self.config.input.mute_button_mode != prev_mode {
            self.needs_save = true;
            GLOBAL_MUTE_HANDLER.set_mode(match self.config.input.mute_button_mode {
                MuteButtonMode::Standard => crate::input::MuteButtonMode::Standard,
                MuteButtonMode::MediaPlayPause => crate::input::MuteButtonMode::MediaPlayPause,
                MuteButtonMode::SmartDouble => crate::input::MuteButtonMode::SmartDouble,
                MuteButtonMode::SmartHold => crate::input::MuteButtonMode::SmartHold,
                        MuteButtonMode::HoldPlayPause => crate::input::MuteButtonMode::HoldPlayPause,
            });
        }
        ui.separator();
        match self.config.input.mute_button_mode {
            MuteButtonMode::Standard => { ui.label("Mute button always toggles microphone in Discord."); }
            MuteButtonMode::MediaPlayPause => { ui.label("Mute button always pauses/plays media."); ui.small("Works with Spotify, YouTube, VLC."); }
            MuteButtonMode::SmartDouble => { ui.label("Single click (< 400 ms) -> MicMute"); ui.label("Double click (< 400 ms) -> Play/Pause"); ui.colored_label(egui::Color32::YELLOW, "(!) 400 ms delay"); }
            MuteButtonMode::SmartHold => { ui.label("Short press (< 500 ms) -> Play/Pause"); ui.label("Long hold (> 500 ms) -> MicMute"); ui.colored_label(egui::Color32::YELLOW, "(!) Requires down/up HID events"); }
            MuteButtonMode::HoldPlayPause => { ui.label("Short press (< 500 ms) -> MicMute"); ui.label("Long hold (> 500 ms) -> Play/Pause"); ui.colored_label(egui::Color32::YELLOW, "(!) Requires down/up HID events"); }
        }
        if ui.button("Test: emulate press").clicked() {
            if self.config.input.mute_button_mode == MuteButtonMode::SmartHold || self.config.input.mute_button_mode == MuteButtonMode::HoldPlayPause {
                ui.label("Test not available for hold modes");
            } else {
                GLOBAL_MUTE_HANDLER.on_mute_toggled(true);
            }
        }
    }

    fn show_settings_voice(&mut self, ui: &mut egui::Ui) {
        ui.heading("Voice Notifications");
        if ui.checkbox(&mut self.config.voice.enabled, "Enable voice").changed() {
            self.needs_save = true;
            log::info!("[GUI] Voice enabled changed to {}", self.config.voice.enabled);
        }
        if self.config.voice.enabled {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut self.config.voice.on_battery_low, "Battery low").changed() { self.needs_save = true; }
                if ui.checkbox(&mut self.config.voice.on_charging, "Charging").changed() { self.needs_save = true; }
                if ui.checkbox(&mut self.config.voice.on_full_charge, "Full charge").changed() { self.needs_save = true; }
            });
            ui.horizontal(|ui| {
                if ui.checkbox(&mut self.config.voice.on_connected, "Connected").changed() { self.needs_save = true; }
                if ui.checkbox(&mut self.config.voice.on_disconnected, "Disconnected").changed() { self.needs_save = true; }
                if ui.checkbox(&mut self.config.voice.on_button_check, "Button check").changed() { self.needs_save = true; }
            });
            if ui.checkbox(&mut self.config.voice.exact_percent, "Exact percent").changed() { self.needs_save = true; }
        }
        if ui.button("Apply Voice Settings").clicked() {
            log::info!("[GUI] Apply Voice Settings clicked");
            self.needs_save = true;
            crate::audio::voice::update_config(self.config.voice.clone());
        }
    }

    fn show_settings_tray_icon(&mut self, ui: &mut egui::Ui) {
        use crate::tray::icon::generate_battery_icon_rgba;
        ui.heading("Tray Icon");
        ui.horizontal(|ui| {
            ui.label("Size:");
            if ui.add(egui::Slider::new(&mut self.tray_icon_config.size, 16..=512)).changed() { self.needs_save = true; }
            ui.label("Font:");
            if ui.add(egui::Slider::new(&mut self.tray_icon_config.font_scale, 1..=20)).changed() { self.needs_save = true; }
        });
        ui.horizontal(|ui| {
            ui.label("Outline:");
            if ui.add(egui::Slider::new(&mut self.tray_icon_config.outline_width, 0..=10)).changed() { self.needs_save = true; }
            ui.label("Border:");
            if ui.add(egui::Slider::new(&mut self.tray_icon_config.border_width, 0..=20)).changed() { self.needs_save = true; }
            ui.label("Gap:");
            if ui.add(egui::Slider::new(&mut self.tray_icon_config.gap_between_digits, 0..=50)).changed() { self.needs_save = true; }
        });

        ui.separator();
        ui.label("Colors");

        ui.group(|ui| {
            ui.label("Charging");
            ui.horizontal(|ui| {
                let mut bg = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.charging.bg[0], self.tray_icon_config.colors.charging.bg[1], self.tray_icon_config.colors.charging.bg[2], self.tray_icon_config.colors.charging.bg[3]);
                let mut fg = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.charging.fg[0], self.tray_icon_config.colors.charging.fg[1], self.tray_icon_config.colors.charging.fg[2], self.tray_icon_config.colors.charging.fg[3]);
                let mut outline = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.charging.outline[0], self.tray_icon_config.colors.charging.outline[1], self.tray_icon_config.colors.charging.outline[2], self.tray_icon_config.colors.charging.outline[3]);
                let mut border = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.charging.border[0], self.tray_icon_config.colors.charging.border[1], self.tray_icon_config.colors.charging.border[2], self.tray_icon_config.colors.charging.border[3]);
                if egui::color_picker::color_edit_button_srgba(ui, &mut bg, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.charging.bg = [bg.r(), bg.g(), bg.b(), bg.a()];
                    self.needs_save = true;
                }
                ui.label("BG");
                if egui::color_picker::color_edit_button_srgba(ui, &mut fg, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.charging.fg = [fg.r(), fg.g(), fg.b(), fg.a()];
                    self.needs_save = true;
                }
                ui.label("FG");
                if egui::color_picker::color_edit_button_srgba(ui, &mut outline, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.charging.outline = [outline.r(), outline.g(), outline.b(), outline.a()];
                    self.needs_save = true;
                }
                ui.label("Out");
                if egui::color_picker::color_edit_button_srgba(ui, &mut border, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.charging.border = [border.r(), border.g(), border.b(), border.a()];
                    self.needs_save = true;
                }
                ui.label("Bdr");
            });
        });

        ui.group(|ui| {
            ui.label("High (>50%)");
            ui.horizontal(|ui| {
                let mut bg = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.high.bg[0], self.tray_icon_config.colors.high.bg[1], self.tray_icon_config.colors.high.bg[2], self.tray_icon_config.colors.high.bg[3]);
                let mut fg = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.high.fg[0], self.tray_icon_config.colors.high.fg[1], self.tray_icon_config.colors.high.fg[2], self.tray_icon_config.colors.high.fg[3]);
                let mut outline = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.high.outline[0], self.tray_icon_config.colors.high.outline[1], self.tray_icon_config.colors.high.outline[2], self.tray_icon_config.colors.high.outline[3]);
                let mut border = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.high.border[0], self.tray_icon_config.colors.high.border[1], self.tray_icon_config.colors.high.border[2], self.tray_icon_config.colors.high.border[3]);
                if egui::color_picker::color_edit_button_srgba(ui, &mut bg, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.high.bg = [bg.r(), bg.g(), bg.b(), bg.a()];
                    self.needs_save = true;
                }
                ui.label("BG");
                if egui::color_picker::color_edit_button_srgba(ui, &mut fg, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.high.fg = [fg.r(), fg.g(), fg.b(), fg.a()];
                    self.needs_save = true;
                }
                ui.label("FG");
                if egui::color_picker::color_edit_button_srgba(ui, &mut outline, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.high.outline = [outline.r(), outline.g(), outline.b(), outline.a()];
                    self.needs_save = true;
                }
                ui.label("Out");
                if egui::color_picker::color_edit_button_srgba(ui, &mut border, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.high.border = [border.r(), border.g(), border.b(), border.a()];
                    self.needs_save = true;
                }
                ui.label("Bdr");
            });
        });

        ui.group(|ui| {
            ui.label("Medium (20-50%)");
            ui.horizontal(|ui| {
                let mut bg = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.medium.bg[0], self.tray_icon_config.colors.medium.bg[1], self.tray_icon_config.colors.medium.bg[2], self.tray_icon_config.colors.medium.bg[3]);
                let mut fg = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.medium.fg[0], self.tray_icon_config.colors.medium.fg[1], self.tray_icon_config.colors.medium.fg[2], self.tray_icon_config.colors.medium.fg[3]);
                let mut outline = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.medium.outline[0], self.tray_icon_config.colors.medium.outline[1], self.tray_icon_config.colors.medium.outline[2], self.tray_icon_config.colors.medium.outline[3]);
                let mut border = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.medium.border[0], self.tray_icon_config.colors.medium.border[1], self.tray_icon_config.colors.medium.border[2], self.tray_icon_config.colors.medium.border[3]);
                if egui::color_picker::color_edit_button_srgba(ui, &mut bg, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.medium.bg = [bg.r(), bg.g(), bg.b(), bg.a()];
                    self.needs_save = true;
                }
                ui.label("BG");
                if egui::color_picker::color_edit_button_srgba(ui, &mut fg, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.medium.fg = [fg.r(), fg.g(), fg.b(), fg.a()];
                    self.needs_save = true;
                }
                ui.label("FG");
                if egui::color_picker::color_edit_button_srgba(ui, &mut outline, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.medium.outline = [outline.r(), outline.g(), outline.b(), outline.a()];
                    self.needs_save = true;
                }
                ui.label("Out");
                if egui::color_picker::color_edit_button_srgba(ui, &mut border, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.medium.border = [border.r(), border.g(), border.b(), border.a()];
                    self.needs_save = true;
                }
                ui.label("Bdr");
            });
        });

        ui.group(|ui| {
            ui.label("Low (<20%)");
            ui.horizontal(|ui| {
                let mut bg = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.low.bg[0], self.tray_icon_config.colors.low.bg[1], self.tray_icon_config.colors.low.bg[2], self.tray_icon_config.colors.low.bg[3]);
                let mut fg = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.low.fg[0], self.tray_icon_config.colors.low.fg[1], self.tray_icon_config.colors.low.fg[2], self.tray_icon_config.colors.low.fg[3]);
                let mut outline = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.low.outline[0], self.tray_icon_config.colors.low.outline[1], self.tray_icon_config.colors.low.outline[2], self.tray_icon_config.colors.low.outline[3]);
                let mut border = egui::Color32::from_rgba_premultiplied(self.tray_icon_config.colors.low.border[0], self.tray_icon_config.colors.low.border[1], self.tray_icon_config.colors.low.border[2], self.tray_icon_config.colors.low.border[3]);
                if egui::color_picker::color_edit_button_srgba(ui, &mut bg, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.low.bg = [bg.r(), bg.g(), bg.b(), bg.a()];
                    self.needs_save = true;
                }
                ui.label("BG");
                if egui::color_picker::color_edit_button_srgba(ui, &mut fg, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.low.fg = [fg.r(), fg.g(), fg.b(), fg.a()];
                    self.needs_save = true;
                }
                ui.label("FG");
                if egui::color_picker::color_edit_button_srgba(ui, &mut outline, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.low.outline = [outline.r(), outline.g(), outline.b(), outline.a()];
                    self.needs_save = true;
                }
                ui.label("Out");
                if egui::color_picker::color_edit_button_srgba(ui, &mut border, egui::color_picker::Alpha::BlendOrAdditive).changed() {
                    self.tray_icon_config.colors.low.border = [border.r(), border.g(), border.b(), border.a()];
                    self.needs_save = true;
                }
                ui.label("Bdr");
            });
        });

        ui.separator();
        ui.heading("Preview");
        ui.horizontal(|ui| {
            let (rgba, w, h) = generate_battery_icon_rgba(&self.tray_icon_config, 75, false);
            let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let texture = ui.ctx().load_texture("preview_75", image, egui::TextureOptions::NEAREST);
            ui.add(egui::Image::new(&texture).fit_to_exact_size([64.0, 64.0].into()));
            let (rgba2, w2, h2) = generate_battery_icon_rgba(&self.tray_icon_config, 42, true);
            let image2 = egui::ColorImage::from_rgba_unmultiplied([w2 as usize, h2 as usize], &rgba2);
            let texture2 = ui.ctx().load_texture("preview_chg", image2, egui::TextureOptions::NEAREST);
            ui.add(egui::Image::new(&texture2).fit_to_exact_size([64.0, 64.0].into()));
        });

        if ui.button("Save Tray Icon Config").clicked() {
            log::info!("[GUI] Save Tray Icon Config clicked");
            self.tray_icon_config.sanitize();
            if let Err(e) = self.tray_icon_config.save(crate::tray::icon::TrayIconConfig::default_path()) {
                log::warn!("Failed to save tray icon config: {}", e);
            }
            if let Some(tray) = &mut self.tray {
                tray.refresh_icon();
                log::info!("[GUI] Tray icon refreshed after save");
            }
        }
        if ui.button("Reset to Default").clicked() {
            log::info!("[GUI] Reset tray icon to default");
            self.tray_icon_config = crate::tray::icon::TrayIconConfig::default();
            self.needs_save = true;
        }
        if ui.button("Apply to Tray Now").clicked() {
            log::info!("[GUI] Apply to Tray Now clicked");
            if let Some(tray) = &mut self.tray {
                tray.update_battery(self.device_state.battery_percent, self.device_state.charging);
            }
        }
    }

    fn show_settings_debug(&mut self, ui: &mut egui::Ui) {
        ui.heading("Debug");
        if ui.checkbox(&mut self.config.debug_logging, "Debug level (verbose)").changed() {
            self.needs_save = true;
            log::info!("[GUI] Debug level changed to {}", self.config.debug_logging);
        }
        if ui.checkbox(&mut self.config.log_to_console, "Log to console").changed() {
            self.needs_save = true;
            log::info!("[GUI] Log to console changed to {}", self.config.log_to_console);
        }
        if ui.checkbox(&mut self.config.log_to_file, "Log to file (hyperx-ngenuity-open.log)").changed() {
            self.needs_save = true;
            log::info!("[GUI] Log to file changed to {}", self.config.log_to_file);
        }
        ui.small("Logging changes require restart.");
    }
}
