use eframe::egui;
use std::sync::Arc;
use crate::audio::debounce::DebouncedEQ;

const BAND_FREQS: [&str; 10] = [
    "32Hz", "64Hz", "125Hz", "250Hz", "500Hz",
    "1kHz", "2kHz", "4kHz", "8kHz", "16kHz"
];

const PRESETS: &[(&str, [f32; 10])] = &[
    ("Flat", [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("FPS", [4.0, 3.0, 1.0, 0.0, -1.0, -2.0, 1.0, 3.0, 4.0, 2.0]),
    ("Music", [2.0, 1.5, 0.5, 0.0, 0.0, 0.5, 1.0, 1.5, 2.0, 1.0]),
    ("Bass Boost", [6.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("Voice", [-2.0, -1.0, 0.0, 2.0, 3.0, 3.0, 2.0, 0.0, -1.0, -2.0]),
];

pub fn show(ui: &mut egui::Ui, bands: &mut [f32; 10], needs_save: &mut bool, apo_available: bool, debounced_eq: &Arc<DebouncedEQ>) {
    ui.heading("🎚️ Эквалайзер");

    if !apo_available {
        ui.colored_label(egui::Color32::RED, "⚠️ Системный EQ не доступен — установите Equalizer APO / eqMac / EasyEffects");
    } else {
        ui.colored_label(egui::Color32::GREEN, "✅ Системный EQ активен (применяется с задержкой 500 мс)");
    }

    ui.label("Системный 10-полосный эквалайзер");
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        for (name, preset_bands) in PRESETS {
            if ui.button(*name).clicked() {
                *bands = *preset_bands;
                *needs_save = true;
                debounced_eq.schedule(*bands);
            }
        }
    });

    ui.separator();

    let mut eq_changed = false;

    egui::Grid::new("eq_grid")
        .num_columns(2)
        .spacing([20.0, 10.0])
        .show(ui, |ui| {
            for (freq, band) in BAND_FREQS.iter().zip(bands.iter_mut()) {
                ui.label(*freq);
                let slider = egui::Slider::new(band, -12.0..=12.0)
                    .show_value(true)
                    .suffix(" dB");
                if ui.add(slider).changed() {
                    *needs_save = true;
                    eq_changed = true;
                }
                ui.end_row();
            }
        });

    if eq_changed {
        debounced_eq.schedule(*bands);
    }

    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("📥 Импортировать пресет").clicked() {
            if let Some(path) = crate::dialog::open_import_dialog() {
                match crate::dialog::PresetFile::load(&path) {
                    Ok(preset) => {
                        *bands = preset.bands;
                        *needs_save = true;
                        debounced_eq.schedule(*bands);
                    }
                    Err(e) => {
                        log::error!("Failed to import preset: {}", e);
                    }
                }
            }
        }
        if ui.button("📤 Экспортировать пресет").clicked() {
            let default_name = "hyperx_preset.hyperx";
            if let Some(path) = crate::dialog::open_export_dialog(default_name) {
                let preset = crate::dialog::PresetFile::new("Custom", *bands);
                if let Err(e) = preset.save(&path) {
                    log::error!("Failed to export preset: {}", e);
                }
            }
        }
        if ui.button("💾 Сохранить как пресет").clicked() {
            // Save to system EQ backend
            #[cfg(target_os = "windows")]
            {
                if let Err(e) = crate::audio::windows::save_preset("custom", bands) {
                    log::error!("Failed to save preset: {}", e);
                }
            }
            #[cfg(target_os = "linux")]
            {
                if let Err(e) = crate::audio::linux::save_preset("hyperx_custom", bands) {
                    log::error!("Failed to save preset: {}", e);
                }
            }
            #[cfg(target_os = "macos")]
            {
                if let Err(e) = crate::audio::macos_eqmac::apply_eq_bands(&bands) {
                    log::error!("Failed to save preset: {}", e);
                }
            }
        }
    });
}
