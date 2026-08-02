use eframe::egui;
use crate::config::{Config, MuteButtonMode};
use crate::device::DeviceState;
use crate::audio::debounce::DebouncedEQ;
use std::sync::Arc;

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
    pub debounced_eq: Arc<DebouncedEQ>,
    pub recording_keybind: bool,
    pub device_rx: Option<std::sync::mpsc::Receiver<crate::DeviceEvent>>,
    pub last_battery_warning: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Dashboard,
    Equalizer,
    Input,
    Discord,
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
            debounced_eq,
            recording_keybind: false,
            device_rx: None,
            last_battery_warning: None,
        }
    }

    pub fn with_tray(mut self, tx: std::sync::mpsc::Sender<crate::tray::TrayCommand>) -> Self {
        self.tray_tx = Some(tx);
        self
    }

    pub fn with_device_receiver(mut self, rx: std::sync::mpsc::Receiver<crate::DeviceEvent>) -> Self {
        self.device_rx = Some(rx);
        self
    }

    fn save_config(&mut self) {
        self.config.audio.eq_bands = self.eq_bands;
        if let Err(e) = self.config.save() {
            eprintln!("Failed to save config: {}", e);
        } else {
            self.needs_save = false;
        }
    }
}

impl eframe::App for HyperXApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Process device events from background thread
        if let Some(rx) = &self.device_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    crate::DeviceEvent::StateChanged(state) => {
                        self.device_state = state;
                    }
                    crate::DeviceEvent::Connected => {
                        self.device_state.connected = true;
                        log::info!("[GUI] Headset connected");
                    }
                    crate::DeviceEvent::Disconnected => {
                        self.device_state.connected = false;
                        log::warn!("[GUI] Headset disconnected");
                    }
                    crate::DeviceEvent::BatteryLow(percent) => {
                        if self.last_battery_warning != Some(percent) {
                            self.last_battery_warning = Some(percent);
                            log::warn!("[GUI] Battery low: {}%", percent);
                            // Could show toast notification here
                        }
                    }
                }
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🎧 HyperX NGENUITY Open");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.device_state.connected {
                        ui.colored_label(egui::Color32::GREEN, "● Подключено");
                    } else {
                        ui.colored_label(egui::Color32::RED, "● Нет подключения");
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, Tab::Dashboard, "📊 Dashboard");
                ui.selectable_value(&mut self.selected_tab, Tab::Equalizer, "🎚️ Эквалайзер");
                ui.selectable_value(&mut self.selected_tab, Tab::Input, "🎮 Ввод");
                ui.selectable_value(&mut self.selected_tab, Tab::Discord, "🔊 Discord");
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "⚙️ Настройки");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Dashboard => self.show_dashboard(ui),
                Tab::Equalizer => eq_tab::show(ui, &mut self.eq_bands, &mut self.needs_save, self.apo_available, &self.debounced_eq),
                Tab::Input => self.show_input_tab(ui),
                Tab::Discord => discord_tab::show(
                    ui,
                    &mut self.config.discord,
                    self.discord_connected,
                    &mut self.needs_save,
                    &mut self.recording_keybind,
                ),
                Tab::Settings => self.show_settings(ui),
            }
        });

        if self.needs_save {
            egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "⚠️ Есть несохранённые изменения");
                    if ui.button("💾 Сохранить").clicked() {
                        self.save_config();
                    }
                });
            });
        }

        // Handle window close -> minimize to tray
        if ctx.input(|i| i.viewport().close_requested()) {
            if let Some(tx) = &self.tray_tx {
                let _ = tx.send(crate::tray::TrayCommand::ShowWindow);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }
}

impl HyperXApp {
    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Состояние гарнитуры");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("🔋 Батарея:");
            let battery = self.device_state.battery_percent as f32 / 100.0;
            let color = if self.device_state.battery_percent > 30 {
                egui::Color32::GREEN
            } else if self.device_state.battery_percent > 15 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            };
            ui.add(
                egui::ProgressBar::new(battery)
                    .text(format!("{}%", self.device_state.battery_percent))
                    .fill(color),
            );
        });

        ui.horizontal(|ui| {
            ui.label("🎤 Микрофон:");
            if self.device_state.muted {
                ui.colored_label(egui::Color32::RED, "🔇 Выключен");
            } else {
                ui.colored_label(egui::Color32::GREEN, "🎙️ Включён");
            }
        });

        ui.horizontal(|ui| {
            ui.label("📶 Сигнал:");
            ui.label(format!("{} dBm", self.device_state.signal_dbm));
        });

        ui.separator();
        ui.heading("Быстрые действия");
        ui.horizontal(|ui| {
            if ui.button(if self.device_state.muted { "🎙️ Включить микрофон" } else { "🔇 Выключить микрофон" }).clicked() {
                // TODO: Send toggle mute command to device
            }
            if ui.button("🔋 Проверить заряд (голос)").clicked() {
                // TODO: Trigger voice prompt
            }
        });
    }

    fn show_input_tab(&mut self, ui: &mut egui::Ui) {
        use crate::input::GLOBAL_MUTE_HANDLER;

        ui.heading("🎮 Настройки кнопки Mute");
        ui.separator();

        ui.label("Режим работы кнопки мьют на гарнитуре:");

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.config.input.mute_button_mode,
                    MuteButtonMode::Standard,
                    "🔇 Стандартно\n(всегда MicMute)"
                );
                ui.selectable_value(
                    &mut self.config.input.mute_button_mode,
                    MuteButtonMode::MediaPlayPause,
                    "⏯️ Всегда Play/Pause"
                );
            });
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.config.input.mute_button_mode,
                    MuteButtonMode::SmartDouble,
                    "🧠 Умный: одиночное = мьют\nдвойное = Play/Pause"
                );
                ui.selectable_value(
                    &mut self.config.input.mute_button_mode,
                    MuteButtonMode::SmartHold,
                    "🧠 Умный: короткое = Play/Pause\nудержание = мьют"
                );
            });
        });

        GLOBAL_MUTE_HANDLER.set_mode(match self.config.input.mute_button_mode {
            MuteButtonMode::Standard => crate::input::MuteButtonMode::Standard,
            MuteButtonMode::MediaPlayPause => crate::input::MuteButtonMode::MediaPlayPause,
            MuteButtonMode::SmartDouble => crate::input::MuteButtonMode::SmartDouble,
            MuteButtonMode::SmartHold => crate::input::MuteButtonMode::SmartHold,
        });

        ui.separator();

        match self.config.input.mute_button_mode {
            MuteButtonMode::Standard => {
                ui.label("Кнопка mute всегда переключает микрофон в Discord.");
            }
            MuteButtonMode::MediaPlayPause => {
                ui.label("Кнопка mute всегда ставит музыку/видео на паузу.");
                ui.small("Работает с Spotify, YouTube, VLC и любыми плеерами.");
            }
            MuteButtonMode::SmartDouble => {
                ui.label("Одиночное нажатие (< 400 мс) → MicMute");
                ui.label("Двойное нажатие (< 400 мс между кликами) → Play/Pause");
                ui.colored_label(egui::Color32::YELLOW, "⚠️ В SmartDouble режиме mute в Discord может задерживаться на 400 мс");
            }
            MuteButtonMode::SmartHold => {
                ui.label("Короткое нажатие (< 500 мс) → Play/Pause");
                ui.label("Длинное удержание (> 500 мс) → MicMute");
                ui.colored_label(egui::Color32::YELLOW, "⚠️ Требует down/up событий от HID (проверьте через Wireshark)");
            }
        }

        if ui.button("🧪 Тест: эмулировать нажатие").clicked() {
            GLOBAL_MUTE_HANDLER.on_mute_toggled(true);
        }

        self.needs_save = true;
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Настройки гарнитуры");
        ui.separator();

        ui.checkbox(&mut self.config.device.sidetone, "Sidetone (слышать себя)");
        if ui.checkbox(&mut self.config.device.voice_prompts, "Голосовые подсказки").changed() {
            self.needs_save = true;
        }

        ui.horizontal(|ui| {
            ui.label("Автоотключение:");
            ui.add(egui::Slider::new(&mut self.config.device.auto_shutdown_minutes, 0..=60)
                .text("мин"));
            if ui.button("Применить").clicked() {
                self.needs_save = true;
            }
        });

        ui.separator();
        ui.heading("Системный эквалайзер");
        ui.checkbox(&mut self.config.audio.system_eq_enabled, "Включить системный EQ");

        #[cfg(target_os = "windows")]
        {
            if self.apo_available {
                ui.colored_label(egui::Color32::GREEN, "✅ Equalizer APO обнаружен");
            } else {
                ui.colored_label(egui::Color32::RED, "❌ Equalizer APO не найден. Установите с https://sourceforge.net/projects/equalizerapo/");
            }
        }

        #[cfg(target_os = "macos")]
        {
            if self.apo_available {
                ui.colored_label(egui::Color32::GREEN, "✅ eqMac обнаружен");
            } else {
                ui.colored_label(egui::Color32::RED, "❌ eqMac не запущен. Установите с https://eqmac.app/");
            }
        }

        ui.label("EQ применяется на уровне ОС (Equalizer APO / EasyEffects / eqMac)");
    }
}
