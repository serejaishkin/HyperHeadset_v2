use crate::tray::icon::{TrayIconConfig, generate_battery_icon_rgba};
use egui::{Color32, TextureHandle};

pub struct TrayIconEditor {
    pub config: TrayIconConfig,
    preview_texture: Option<TextureHandle>,
    preview_percent: u8,
    preview_charging: bool,
}

impl Default for TrayIconEditor {
    fn default() -> Self {
        Self {
            config: TrayIconConfig::load_or_create(),
            preview_texture: None,
            preview_percent: 75,
            preview_charging: false,
        }
    }
}

impl TrayIconEditor {
    pub fn show(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Tray Icon Settings");
        ui.separator();

        ui.collapsing("📐 Dimensions", |ui| {
            ui.add(egui::Slider::new(&mut self.config.size, 32..=512).text("Icon Size (px)"));
            ui.add(egui::Slider::new(&mut self.config.font_scale, 1..=32).text("Font Scale"));
            ui.add(egui::Slider::new(&mut self.config.outline_width, 0..=10).text("Outline Width"));
            ui.add(egui::Slider::new(&mut self.config.border_width, 0..=20).text("Border Width"));
            ui.add(egui::Slider::new(&mut self.config.gap_between_digits, 0..=20).text("Digit Gap"));
        });

        ui.collapsing("🎨 Colors", |ui| {
            Self::color_scheme_editor(ui, "Charging", &mut self.config.colors.charging);
            Self::color_scheme_editor(ui, "High (>50%)", &mut self.config.colors.high);
            Self::color_scheme_editor(ui, "Medium (20–50%)", &mut self.config.colors.medium);
            Self::color_scheme_editor(ui, "Low (<20%)", &mut self.config.colors.low);
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Preview:");
            ui.add(egui::Slider::new(&mut self.preview_percent, 0..=100).text("%"));
            ui.checkbox(&mut self.preview_charging, "Charging");
        });

        self.update_preview(ctx);
        if let Some(tex) = &self.preview_texture {
            let size = tex.size_vec2();
            ui.image((tex.id(), size * 2.0));
        }

        ui.separator();

        if ui.button("💾 Save Settings").clicked() {
            let path = TrayIconConfig::default_path();
            if let Err(e) = self.config.save(&path) {
                log::error!("[TrayIconEditor] Save failed: {}", e);
            } else {
                log::info!("[TrayIconEditor] Settings saved to {:?}", path);
            }
        }
    }

    fn color_scheme_editor(ui: &mut egui::Ui, label: &str, colors: &mut crate::tray::icon::IconColors) {
        ui.group(|ui| {
            ui.label(label);
            Self::color_edit(ui, "Background", &mut colors.bg);
            Self::color_edit(ui, "Foreground", &mut colors.fg);
            Self::color_edit(ui, "Outline", &mut colors.outline);
            Self::color_edit(ui, "Border", &mut colors.border);
        });
    }

    fn color_edit(ui: &mut egui::Ui, label: &str, rgba: &mut [u8; 4]) {
        ui.horizontal(|ui| {
            ui.label(label);
            let mut c32 = Color32::from_rgba_premultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
            ui.color_edit_button_srgba(&mut c32);
            rgba[0] = c32.r();
            rgba[1] = c32.g();
            rgba[2] = c32.b();
            rgba[3] = c32.a();
        });
    }

    fn update_preview(&mut self, ctx: &egui::Context) {
        let (rgba, w, h) = generate_battery_icon_rgba(&self.config, self.preview_percent, self.preview_charging);
        let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let handle = ctx.load_texture("tray_preview", image, Default::default());
        self.preview_texture = Some(handle);
    }
}
