import re

# === 1. gui/mod.rs ===
with open('src/gui/mod.rs', 'r') as f:
    gui = f.read()

# 1a. Add mic volume + fix expand button in show_compact_ui
old_compact = '''    fn show_compact_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(5.0);
                let icon = if self.device_state.charging { "⚡" } else { "🔋" };
                let color = if self.device_state.battery_percent > 30 { egui::Color32::GREEN }
                    else if self.device_state.battery_percent > 15 { egui::Color32::YELLOW }
                    else { egui::Color32::RED };
                ui.colored_label(color, format!("{} {}%", icon, self.device_state.battery_percent));
                ui.add(egui::ProgressBar::new(self.device_state.battery_percent as f32 / 100.0).desired_width(180.0).fill(color));
                ui.add_space(5.0);
                let mic_icon = if self.device_state.muted { "🔇" } else { "🎙️" };
                let mic_text = if self.device_state.muted { self.i18n.t("MUTE") } else { self.i18n.t("MIC ON") };
                ui.label(format!("{} {}", mic_icon, mic_text));
                ui.add_space(5.0);
                ui.label(self.i18n.t("VOL"));
                let mut vol = self.volume;
                ui.add(egui::Slider::new(&mut vol, 0.0..=100.0).show_value(true).text(""));
                if vol != self.volume {
                    self.volume = vol;
                    if let Some(ref controller) = self.volume_controller {
                        controller.set_master_volume(vol);
                    }
                }
                ui.add_space(5.0);
                if ui.button(format!("⛶ {}", self.i18n.t("Expand"))).clicked() {
                    self.config.compact_mode = false;
                    self.needs_save = true;
                }
            });
        });
    }'''

new_compact = '''    fn show_compact_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                // Battery
                let icon = if self.device_state.charging { "⚡" } else { "🔋" };
                let color = if self.device_state.battery_percent > 30 { egui::Color32::GREEN }
                    else if self.device_state.battery_percent > 15 { egui::Color32::YELLOW }
                    else { egui::Color32::RED };
                ui.colored_label(color, format!("{} {}%", icon, self.device_state.battery_percent));
                ui.add(egui::ProgressBar::new(self.device_state.battery_percent as f32 / 100.0).desired_width(180.0).fill(color));
                ui.add_space(4.0);
                // Mic status
                let mic_icon = if self.device_state.muted { "🔇" } else { "🎙️" };
                let mic_text = if self.device_state.muted { self.i18n.t("MUTE") } else { self.i18n.t("MIC ON") };
                ui.label(format!("{} {}", mic_icon, mic_text));
                ui.add_space(4.0);
                // Master volume
                ui.label(self.i18n.t("VOL"));
                let mut vol = self.volume;
                ui.add(egui::Slider::new(&mut vol, 0.0..=100.0).show_value(true).text(""));
                if vol != self.volume {
                    self.volume = vol;
                    if let Some(ref controller) = self.volume_controller {
                        controller.set_master_volume(vol);
                    }
                }
                ui.add_space(4.0);
                // Mic volume
                ui.label(self.i18n.t("MIC"));
                let mut mic_vol = self.mic_volume;
                ui.add(egui::Slider::new(&mut mic_vol, 0.0..=100.0).show_value(true).text(""));
                if mic_vol != self.mic_volume {
                    self.mic_volume = mic_vol;
                    if let Some(ref controller) = self.volume_controller {
                        controller.set_microphone_volume(mic_vol);
                    }
                }
                ui.add_space(4.0);
                // Expand button
                if ui.button(format!("⛶ {}", self.i18n.t("Expand"))).clicked() {
                    self.config.compact_mode = false;
                    self.needs_save = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([980.0, 600.0].into()));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
                }
            });
        });
    }'''

if old_compact in gui:
    gui = gui.replace(old_compact, new_compact)
    print("Fixed show_compact_ui")
else:
    print("WARN: show_compact_ui pattern not found")

# 1b. Fix compact button in TopPanel — also resize back
old_btn = '''                    if ui.button("⛶").clicked() {
                        self.config.compact_mode = !self.config.compact_mode;
                        self.needs_save = true;
                    }'''
new_btn = '''                    if ui.button("⛶").clicked() {
                        self.config.compact_mode = !self.config.compact_mode;
                        self.needs_save = true;
                        if !self.config.compact_mode {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([980.0, 600.0].into()));
                        }
                    }'''
if old_btn in gui:
    gui = gui.replace(old_btn, new_btn)
    print("Fixed compact button")
else:
    print("WARN: compact button pattern not found")

with open('src/gui/mod.rs', 'w') as f:
    f.write(gui)

# === 2. main.rs — убрать пустое консольное окно ===
with open('src/main.rs', 'r') as f:
    main = f.read()

old_main = '// #![cfg_attr(target_os = "windows", windows_subsystem = "windows")] temporarily removed for debug'
new_main = '#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]'
if old_main in main:
    main = main.replace(old_main, new_main)
    with open('src/main.rs', 'w') as f:
        f.write(main)
    print("Fixed windows console window")
else:
    print("WARN: windows_subsystem pattern not found")

print("Done! Run: cargo build --release")