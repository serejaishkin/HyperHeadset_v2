//! Logging that always reaches a file: the Windows builds have no console.

use crate::config::Config;

/// Initialise env_logger. The log goes to `hyperx-ngenuity-open.log` next to the exe (falling back
/// to the temp dir) regardless of the saved `log_to_file` setting; panics are written there too.
#[cfg(target_os = "windows")]
fn attach_parent_console() -> Option<std::fs::File> {
    // GUI-subsystem exes have no console; when started from a terminal, attach to it so that
    // log output (and println!) shows up there.
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(pid: u32) -> i32;
    }
    if unsafe { AttachConsole(0xFFFF_FFFF) } == 0 {
        return None; // ATTACH_PARENT_PROCESS failed: not started from a console
    }
    std::fs::OpenOptions::new().write(true).open("CONOUT$").ok()
}

pub fn init(config: &Config) {
    #[cfg(target_os = "windows")]
    let console = attach_parent_console().map(std::sync::Mutex::new);
    #[cfg(not(target_os = "windows"))]
    let console: Option<std::sync::Mutex<std::fs::File>> = None;
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(if config.debug_logging { log::LevelFilter::Debug } else { log::LevelFilter::Info });

    let log_to_console = config.log_to_console;
    let mut log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("hyperx-ngenuity-open.log")))
        .unwrap_or_else(|| std::path::PathBuf::from("hyperx-ngenuity-open.log"));
    if std::fs::OpenOptions::new().create(true).append(true).open(&log_path).is_err() {
        log_path = std::env::temp_dir().join("hyperx-ngenuity-open.log");
    }
    if std::fs::metadata(&log_path).map(|m| m.len() > 5_000_000).unwrap_or(false) {
        let _ = std::fs::remove_file(&log_path);
    }
    let panic_path = log_path.clone();
    let shown_path = log_path.clone();
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&panic_path) {
            let _ = std::io::Write::write_all(&mut f, format!("[PANIC] {}\n", info).as_bytes());
        }
    }));
    builder.format(move |buf, record| {
        let line = format!("[{}] {} - {}\n", record.level(), record.target(), record.args());
        if let Some(con) = &console {
            if let Ok(mut c) = con.lock() {
                let _ = std::io::Write::write_all(&mut *c, line.as_bytes());
            }
        } else if log_to_console {
            let _ = std::io::Write::write_all(buf, line.as_bytes());
        }
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
            let _ = std::io::Write::write_all(&mut file, line.as_bytes());
        }
        Ok(())
    });
    let _ = builder.try_init();
    log::info!("[Main] Log started (v{}) -> {}", env!("CARGO_PKG_VERSION"), shown_path.display());
}
