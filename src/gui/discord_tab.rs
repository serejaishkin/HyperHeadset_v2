use eframe::egui;
use crate::config::{DiscordConfig, DiscordMode};
use crate::hotkey::GlobalHotkeyCapture;
use std::sync::Arc;

pub fn show(
    ui: &mut egui::Ui,
    config: &mut DiscordConfig,
    connected: bool,
    needs_save: &mut bool,
    recording_keybind: &mut bool,
) {
    ui.heading("🔊 Discord интеграция");
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Режим:");
        ui.selectable_value(&mut config.mode, DiscordMode::None, "❌ Отключено");
        ui.selectable_value(&mut config.mode, DiscordMode::Keybind, "⌨️ Клавиша");
        ui.selectable_value(&mut config.mode, DiscordMode::Direct, "🔌 Прямая (RPC)");
    });

    if ui.data(|data| data.get_temp::<DiscordMode>(egui::Id::new("discord_mode_prev"))) != Some(config.mode) {
        *needs_save = true;
    }

    ui.separator();

    match config.mode {
        DiscordMode::None => {
            ui.label("Discord интеграция отключена.");
            ui.small("Кнопка mute на гарнитуре не будет влиять на Discord.");
        }

        DiscordMode::Keybind => {
            ui.label("При нажатии кнопки mute на гарнитуре будет эмулироваться клавиша:");

            ui.horizontal(|ui| {
                let keybind = config.keybind.get_or_insert_with(|| "F20".to_string());

                if *recording_keybind {
                    ui.colored_label(egui::Color32::YELLOW, "🎙️ Нажмите комбинацию клавиш...");

                    // Poll for captured keybind
                    // TODO: Integrate with GlobalHotkeyCapture singleton
                } else {
                    ui.text_edit_singleline(keybind);
                }

                if ui.button(if *recording_keybind { "❌ Отмена" } else { "📝 Записать клавишу" }).clicked() {
                    *recording_keybind = !*recording_keybind;
                    if *recording_keybind {
                        // TODO: Start global hotkey capture
                    }
                }
            });

            ui.small("💡 Назначьте эту же клавишу в Discord → Настройки → Горячие клавиши → Переключить мьют");
            ui.small("Доступные: F13-F24, MediaMute, MediaPlayPause, Ctrl+Shift+M, и др.");
        }

        DiscordMode::Direct => {
            ui.group(|ui| {
                ui.label("Discord Rich Presence + двусторонний mute");

                if connected {
                    ui.colored_label(egui::Color32::GREEN, "● Подключено к Discord IPC");
                } else {
                    ui.colored_label(egui::Color32::RED, "● Не подключено");
                    if ui.button("Подключить").clicked() {
                        // TODO: Initialize Discord IPC + WebSocket
                    }
                }

                ui.horizontal(|ui| {
                    ui.label("Application ID:");
                    ui.text_edit_singleline(&mut config.direct.app_id);
                });

                ui.checkbox(&mut config.direct.show_battery, "Показывать заряд в статусе Discord");
                ui.checkbox(&mut config.direct.show_mute_status, "Показывать статус микрофона");

                ui.separator();
                ui.label("Статус:");
                if connected {
                    ui.label("🎧 HyperX Cloud II Wireless");
                    ui.label("🔋 Батарея: 85%");
                    ui.label("🎤 Микрофон: включён");
                }
            });

            ui.separator();
            ui.colored_label(
                egui::Color32::YELLOW,
                "⚠️ Двусторонний sync (mute в Discord → гарнитура) через локальный RPC WebSocket"
            );
            ui.hyperlink_to("Как получить App ID", "https://discord.com/developers/applications");
        }
    }
}
