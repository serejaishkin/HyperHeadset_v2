//! macOS equalizer backend via the open-source `eqtune` CLI
//! (system-wide parametric EQ on a Core Audio process tap, macOS 14.2+).
use anyhow::{anyhow, bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

const BAND_FREQUENCIES: [f32; 10] = [
    32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];
const Q: f32 = 1.41;

fn find_binary(name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".cargo/bin"));
    }
    dirs.push("/opt/homebrew/bin".into());
    dirs.push("/usr/local/bin".into());
    dirs.into_iter().map(|d| d.join(name)).find(|p| p.is_file())
}

pub fn is_installed() -> bool {
    find_binary("eqtune").is_some()
}

fn check_macos_version() -> Result<()> {
    let out = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .context("Не удалось определить версию macOS (sw_vers)")?;
    let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let mut it = ver.split('.').map(|s| s.parse::<u32>().unwrap_or(0));
    let (major, minor) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
    if (major, minor) < (14, 2) {
        bail!("eqtune требует macOS 14.2 или новее (сейчас {ver})");
    }
    Ok(())
}

fn run(bin: &PathBuf, args: &[String]) -> Result<()> {
    let out = Command::new(bin)
        .args(args)
        .output()
        .with_context(|| format!("Не удалось запустить {}", bin.display()))?;
    if !out.status.success() {
        bail!(
            "eqtune {} завершился с ошибкой: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Sets 10 peaking bands + preamp through `eqtune` and starts the engine.
pub fn apply_eq_bands(bands: &[f32; 10]) -> Result<()> {
    check_macos_version()?;
    let bin = find_binary("eqtune").ok_or_else(|| {
        anyhow!("eqtune не найден. Установите его (`cargo install eqtune`) или вызовите install()")
    })?;
    // Daemon must be registered with launchd; `install` is documented to be safe to re-run.
    run(&bin, &["install".into()])?;
    for (freq, gain) in BAND_FREQUENCIES.iter().zip(bands.iter()) {
        let gain = gain.clamp(-24.0, 24.0);
        run(
            &bin,
            &[
                "band".into(),
                format!("{freq}"),
                format!("{gain:.1}"),
                format!("{Q}"),
            ],
        )?;
    }
    let max_pos = bands.iter().cloned().fold(0.0_f32, f32::max).min(24.0);
    run(&bin, &["preamp".into(), format!("{:.1}", -max_pos)])?;
    run(&bin, &["on".into()])?;
    log::info!(
        "[MacEQ] eqtune: 10 bands applied, preamp {:.1} dB",
        -max_pos
    );
    Ok(())
}
