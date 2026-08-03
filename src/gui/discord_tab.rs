use eframe::egui;
use crate::config::{DiscordConfig, DiscordMode};
use std::sync::Arc;

pub fn show(
    ui: &mut egui::Ui,
    config: &mut DiscordConfig,
    connected: bool,
    needs_save: &mut bool,
    recording_keybind: &mut bool,
    keybind_capture: &mut Option<Arc<crate::hotkey::GlobalHotkeyCapture>>,
) {
    ui.heading("🔊 Discord интеграция");
    ui.separator();

    let prev_mode = config.mode;
    ui.horizontal(|ui| {
        ui.label("Режим:");
        ui.selectable_value(&mut config.mode, DiscordMode::None, "❌ Отключено");
        ui.selectable_value(&mut config.mode, DiscordMode::Keybind, "⌨️ Клавиша");
        ui.selectable_value(&mut config.mode, DiscordMode::Direct, "🔌 Прямая (RPC)");
    });
    if config.mode != prev_mode {
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
                } else {
                    ui.add(egui::TextEdit::singleline(keybind).desired_width(120.0));
                }

                if ui.button(if *recording_keybind { "❌ Отмена" } else { "📝 Записать клавишу" }).clicked() {
                    if *recording_keybind {
                        if let Some(capture) = keybind_capture {
                            capture.cancel();
                        }
                        *recording_keybind = false;
                        *keybind_capture = None;
                    } else {
                        println!("[DEBUG DISCORD] Starting keybind recording...");
                        *recording_keybind = true;
                        let capture = Arc::new(crate::hotkey::GlobalHotkeyCapture::new());
                        crate::hotkey::spawn_capture(capture.clone());
                        capture.start_recording();
                        *keybind_capture = Some(capture);
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
                    let can_connect = !config.direct.app_id.is_empty();
                    if ui.add_enabled(can_connect, egui::Button::new("Подключить")).clicked() {
                        // TODO
                    }
                    if !can_connect {
                        ui.small("Введите App ID для подключения (или используйте режим 'Клавиша')");
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("App ID (опционально):");
                    ui.text_edit_singleline(&mut config.direct.app_id);
                });
                ui.small("Оставьте пустым, если не нужен статус 'Playing' в профиле Discord.");
                ui.checkbox(&mut config.direct.show_battery, "Показывать заряд в статусе Discord");
                ui.checkbox(&mut config.direct.show_mute_status, "Показывать статус микрофона");
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