#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui;
use std::fs::OpenOptions;
use std::io::Write;

fn setup_file_logging() {
    let log_path = std::env::temp_dir().join("hyperx-ngenuity-open.log");
    let _ = OpenOptions::new().create(true).append(true).open(&log_path);
    env_logger::Builder::from_default_env()
        .format(move |buf, record| {
            let _ = writeln!(buf, "[{}] {} - {}", record.level(), record.target(), record.args());
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let _ = writeln!(file, "[{}] {} - {}", record.level(), record.target(), record.args());
            }
            Ok(())
        })
        .init();
}

fn main() {
    setup_file_logging();
    log::info!("=== Application starting ===");

    let result = std::panic::catch_unwind(|| {
        run_app();
    });

    if let Err(e) = result {
        let msg = if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else {
            "Unknown panic".to_string()
        };
        log::error!("PANIC: {}", msg);
        // Показываем MessageBox на Windows
        #[cfg(target_os = "windows")]
        unsafe {
            let _ = windows::Win32::System::Threading::MessageBoxA(
                None,
                &windows::core::HSTRING::from(&format!("Crash on startup:\\n{}\\n\\nLogs: %TEMP%\\\\hyperx-ngenuity-open.log", msg)),
                &windows::core::HSTRING::from("HyperX NGENUITY Open - Error"),
                windows::Win32::UI::WindowsAndMessaging::MB_OK | windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
        std::process::exit(1);
    }
}

fn run_app() {
    let config = hyperx_ngenuity_open::config::Config::load_or_create();
    hyperx_ngenuity_open::audio::voice::update_config(config.voice.clone());

    log::info!("Config loaded, voice config updated");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    log::info!("Starting eframe...");

    let _ = eframe::run_native(
        "HyperX NGENUITY Open",
        native_options,
        Box::new(|cc| {
            log::info!("Creating app instance...");
            let device_state = hyperx_ngenuity_open::device::DeviceState::default();
            let debounced_eq = std::sync::Arc::new(hyperx_ngenuity_open::audio::debounce::DebouncedEQ::new(
                std::sync::mpsc::channel().0,
            ));
            let app = hyperx_ngenuity_open::gui::HyperXApp::new(config, device_state, false, debounced_eq);
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    );
}
