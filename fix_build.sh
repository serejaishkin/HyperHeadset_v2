cd /workspaces/HyperHeadsetv2

# 1. Restore gui/mod.rs from last WORKING commit (before the mess)
git checkout d6780f3 -- src/gui/mod.rs

# 2. Apply clean patch
cat > /tmp/patch_gui.py << 'PYEOF'
import re

with open("src/gui/mod.rs", "r") as f:
    gui = f.read()

# A. SettingsTab enum
gui = gui.replace(
    '#[derive(Debug, Clone, Copy, PartialEq)]\npub enum Tab {',
    '''#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsTab {
    Headset,
    Voice,
    TrayIcon,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {'''
)

# B. Fields
gui = gui.replace(
    '    pub show_discord_panel: bool,',
    '''    pub show_discord_panel: bool,
    pub selected_settings_tab: SettingsTab,
    pub tray_icon_config: TrayIconConfig,'''
)

# C. Init
gui = gui.replace(
    '            show_discord_panel: false,',
    '''            show_discord_panel: false,
            selected_settings_tab: SettingsTab::Headset,
            tray_icon_config: TrayIconConfig::load_or_create(),'''
)

# D. Replace show_settings with switch + show_settings_headset
old = '''    fn show_settings(&mut self, ui: &mut egui::Ui) {
        use crate::input::GLOBAL_MUTE_HANDLER;

        egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Headset Settings");'''

new = '''    fn show_settings(&mut self, ui: &mut egui::Ui) {
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

        ui.heading("Headset Settings");'''

gui = gui.replace(old, new)

# E. Replace end of show_settings (old ScrollArea close) with close of show_settings_headset + new tabs
old_end = '''        ui.label("EQ is applied at OS level");
        }); // ScrollArea end
    }'''

new_end = '''        ui.label("EQ is applied at OS level");
    }

    fn show_settings_voice(&mut self, ui: &mut egui::Ui) {
        ui.heading("Voice Notifications");
        if ui.checkbox(&mut self.config.voice.enabled, "Enable voice").changed() { self.needs_save = true; }
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
            self.needs_save = true;
            crate::audio::voice::update_config(self.config.voice.clone());
        }
    }

    fn show_settings_tray_icon(&mut self, ui: &mut egui::Ui) {
        use crate::tray::icon::generate_battery_icon_rgba;
        ui.heading("Tray Icon");
        ui.horizontal(|ui| {
            ui.label("Size:");
            if ui.add(egui::DragValue::new(&mut self.tray_icon_config.size).speed(1).range(16..=512)).changed() { self.needs_save = true; }
            ui.label("Font:");
            if ui.add(egui::DragValue::new(&mut self.tray_icon_config.font_scale).speed(1).range(1..=20)).changed() { self.needs_save = true; }
        });
        ui.horizontal(|ui| {
            ui.label("Outline:");
            if ui.add(egui::DragValue::new(&mut self.tray_icon_config.outline_width).speed(1).range(0..=10)).changed() { self.needs_save = true; }
            ui.label("Border:");
            if ui.add(egui::DragValue::new(&mut self.tray_icon_config.border_width).speed(1).range(0..=20)).changed() { self.needs_save = true; }
            ui.label("Gap:");
            if ui.add(egui::DragValue::new(&mut self.tray_icon_config.gap_between_digits).speed(1).range(0..=50)).changed() { self.needs_save = true; }
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
            self.tray_icon_config.sanitize();
            if let Err(e) = self.tray_icon_config.save(crate::tray::icon::TrayIconConfig::default_path()) {
                log::warn!("Failed to save tray icon config: {}", e);
            }
            if let Some(tray) = &mut self.tray {
                tray.refresh_icon();
            }
        }
        if ui.button("Reset to Default").clicked() {
            self.tray_icon_config = crate::tray::icon::TrayIconConfig::default();
            self.needs_save = true;
        }
        if ui.button("Apply to Tray Now").clicked() {
            if let Some(tray) = &mut self.tray {
                tray.update_battery(self.device_state.battery_percent, self.device_state.charging);
            }
        }
    }

    fn show_settings_debug(&mut self, ui: &mut egui::Ui) {
        ui.heading("Debug");
        if ui.checkbox(&mut self.config.debug_logging, "Debug level (verbose)").changed() {
            self.needs_save = true;
        }
        if ui.checkbox(&mut self.config.log_to_console, "Log to console").changed() {
            self.needs_save = true;
        }
        if ui.checkbox(&mut self.config.log_to_file, "Log to file (hyperx-ngenuity-open.log)").changed() {
            self.needs_save = true;
        }
        ui.small("Logging changes require restart.");
    }'''

gui = gui.replace(old_end, new_end)

with open("src/gui/mod.rs", "w") as f:
    f.write(gui)
print("[OK] gui/mod.rs")
PYEOF

python3 /tmp/patch_gui.py

# 3. Fix Linux tray
cat > /tmp/patch_linux.py << 'PYEOF'
with open('src/tray/linux.rs', 'r') as f:
    linux = f.read()
if '_rgba' in linux:
    start = linux.find('    pub fn refresh_icon')
    end = linux.find('    pub fn update_battery', start)
    if start != -1 and end != -1:
        linux = linux[:start] + linux[end:]
if 'pub fn refresh_icon' not in linux:
    linux = linux.replace('    pub fn poll(&self) {}', '    pub fn refresh_icon(&self) {}\n\n    pub fn poll(&self) {}')
with open('src/tray/linux.rs', 'w') as f:
    f.write(linux)
print('[OK] linux.rs')
PYEOF

python3 /tmp/patch_linux.py

# 4. Fix macOS tray
cat > /tmp/patch_macos.py << 'PYEOF'
with open('src/tray/macos.rs', 'r') as f:
    macos = f.read()
if 'pub fn refresh_icon' not in macos:
    macos = macos.replace('    pub fn poll(&mut self) {}', '    pub fn refresh_icon(&mut self) {\n        let percent = *self.current_percent.lock().unwrap();\n        let charging = *self.current_charging.lock().unwrap();\n        let (rgba, w, h) = generate_battery_icon_rgba(&self.icon_config, percent, charging);\n        if let Ok(icon) = Icon::from_rgba(rgba, w, h) {\n            let _ = self.tray_icon.set_icon(Some(icon));\n        }\n    }\n\n    pub fn poll(&mut self) {}')
with open('src/tray/macos.rs', 'w') as f:
    f.write(macos)
print('[OK] macos.rs')
PYEOF

python3 /tmp/patch_macos.py

# 5. Commit and push
git add -A && git commit -m "Fix: deduplicate show_settings_headset, clean sub-tabs" && git push origin main
