#!/usr/bin/env python3
import os, shutil, subprocess
from pathlib import Path

REPO = Path("/workspaces/HyperHeadsetv2")

def read(f): return f.read_text(encoding="utf-8")
def write(f, t): f.write_text(t, encoding="utf-8"); print(f"[OK] {f}")

# ═══════════════════════════════════════════════════════
# 1. Cargo.toml — убираем системный OpenSSL (rustls)
# ═══════════════════════════════════════════════════════
cargo = REPO / "Cargo.toml"
txt = read(cargo)
txt = txt.replace(
    'reqwest = { version = "0.12", features = ["json"] }',
    'reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }'
)
write(cargo, txt)

# ═══════════════════════════════════════════════════════
# 2. voice.rs — структура с комментариями
#    Закомментируй строку в embedded:: и соответствующую в match,
#    если файла assets/voice/bat_XXX.wav ещё нет
# ═══════════════════════════════════════════════════════
voice = REPO / "src/audio/voice.rs"
txt = read(voice)

txt = txt.replace(
    '''mod embedded {
    pub const BAT_001: &[u8] = include_bytes!("../../assets/voice/bat_001.wav");
    pub const BAT_005: &[u8] = include_bytes!("../../assets/voice/bat_005.wav");
    pub const BAT_010: &[u8] = include_bytes!("../../assets/voice/bat_010.wav");
    pub const BAT_020: &[u8] = include_bytes!("../../assets/voice/bat_020.wav");
    pub const BAT_050: &[u8] = include_bytes!("../../assets/voice/bat_050.wav");
    pub const BAT_100: &[u8] = include_bytes!("../../assets/voice/bat_100.wav");
    pub const CHARGING: &[u8] = include_bytes!("../../assets/voice/charging.wav");
    pub const FULL_CHARGE: &[u8] = include_bytes!("../../assets/voice/full_charge.wav");
    pub const LOW_BATTERY: &[u8] = include_bytes!("../../assets/voice/low_battery.wav");
}''',
    '''mod embedded {
    // === БАТАРЕЯ ===
    // ЗАКОММЕНТИРУЙ строку, если файла assets/voice/bat_XXX.wav ещё нет
    pub const BAT_001: &[u8] = include_bytes!("../../assets/voice/bat_001.wav");
    pub const BAT_005: &[u8] = include_bytes!("../../assets/voice/bat_005.wav");
    pub const BAT_010: &[u8] = include_bytes!("../../assets/voice/bat_010.wav");
    pub const BAT_020: &[u8] = include_bytes!("../../assets/voice/bat_020.wav");
    pub const BAT_050: &[u8] = include_bytes!("../../assets/voice/bat_050.wav");
    pub const BAT_100: &[u8] = include_bytes!("../../assets/voice/bat_100.wav");
    
    // === СТАТУСЫ ===
    pub const CHARGING: &[u8] = include_bytes!("../../assets/voice/charging.wav");
    pub const FULL_CHARGE: &[u8] = include_bytes!("../../assets/voice/full_charge.wav");
    pub const LOW_BATTERY: &[u8] = include_bytes!("../../assets/voice/low_battery.wav");
}'''
)

txt = txt.replace(
    '''fn nearest_battery(percent: u8) -> &'static [u8] {
    match percent {
        1 => embedded::BAT_001,
        5 => embedded::BAT_005,
        10 => embedded::BAT_010,
        20 => embedded::BAT_020,
        50 => embedded::BAT_050,
        100 => embedded::BAT_100,
        0..=15 => embedded::BAT_010,
        16..=35 => embedded::BAT_020,
        36..=65 => embedded::BAT_050,
        _ => embedded::BAT_100,
    }
}''',
    '''fn nearest_battery(percent: u8) -> &'static [u8] {
    match percent {
        // ЗАКОММЕНТИРУЙ ветку, если соответствующий BAT_XXX закомментирован выше
        1 => embedded::BAT_001,
        5 => embedded::BAT_005,
        10 => embedded::BAT_010,
        20 => embedded::BAT_020,
        50 => embedded::BAT_050,
        100 => embedded::BAT_100,
        
        // Fallback на ближайший существующий
        0..=15 => embedded::BAT_010,
        16..=35 => embedded::BAT_020,
        36..=65 => embedded::BAT_050,
        _ => embedded::BAT_100,
    }
}'''
)
write(voice, txt)

# ═══════════════════════════════════════════════════════
# 3. gui/mod.rs — фикс моргания в компактном режиме
# ═══════════════════════════════════════════════════════
gui = REPO / "src/gui/mod.rs"
txt = read(gui)

txt = txt.replace(
    '''    fn toggle_compact_mode(&mut self, ctx: &egui::Context) {
        self.compact_mode = !self.compact_mode;
        let new_size = if self.compact_mode {
            egui::vec2(320.0, 200.0)
        } else {
            egui::vec2(900.0, 600.0)
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(new_size));
    }''',
    '''    fn toggle_compact_mode(&mut self, ctx: &egui::Context) {
        self.compact_mode = !self.compact_mode;
        let new_size = if self.compact_mode {
            egui::vec2(320.0, 200.0)
        } else {
            egui::vec2(900.0, 600.0)
        };
        
        // FIX: моргание при resize на Wayland/X11 + NVIDIA
        // Форсируем перерисовку до и после смены размера
        ctx.request_repaint();
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(new_size));
        ctx.request_repaint();
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }'''
)
write(gui, txt)

# ═══════════════════════════════════════════════════════
# 4. Сборка
# ═══════════════════════════════════════════════════════
print("\n[BUILD] cargo update...")
subprocess.run(["cargo", "update"], cwd=REPO, check=True)

print("[BUILD] cargo build --release...")
subprocess.run(["cargo", "build", "--release"], cwd=REPO, check=True)

# ═══════════════════════════════════════════════════════
# 5. AppImage
# ═══════════════════════════════════════════════════════
print("[BUILD] AppImage...")
appdir = REPO / "HyperX-NGENUITY-Open.AppDir"
if appdir.exists(): shutil.rmtree(appdir)
(appdir / "usr/bin").mkdir(parents=True)
(appdir / "usr/share").mkdir(parents=True)
(appdir / "usr/share/udev").mkdir(parents=True)

shutil.copy(REPO / "target/release/hyperx-ngenuity-open", appdir / "usr/bin/")
if (REPO / "lang").exists():
    shutil.copytree(REPO / "lang", appdir / "usr/share/lang", dirs_exist_ok=True)
shutil.copy(REPO / "99-HyperHeadset.rules", appdir / "usr/share/udev/")
icon_src = REPO / "assets/icon.png"
if icon_src.exists():
    shutil.copy(icon_src, appdir / "hyperx-ngenuity-open.png")
else:
    subprocess.run(["convert", "-size", "256x256", "xc:red", str(appdir / "hyperx-ngenuity-open.png")])

(appdir / "hyperx-ngenuity-open.desktop").write_text("""[Desktop Entry]
Name=HyperX NGENUITY Open
Exec=hyperx-ngenuity-open
Type=Application
Icon=hyperx-ngenuity-open
Categories=AudioVideo;Audio;
""", encoding="utf-8")

(appdir / "AppRun").write_text("""#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"
RULES_DST="/etc/udev/rules.d/99-HyperHeadset.rules"
if [ ! -f "$RULES_DST" ]; then
    echo ""
    echo "⚠️  Нужны права на USB-гарнитуру. Выполните ОДИН раз:"
    echo "   sudo cp '${HERE}/usr/share/udev/99-HyperHeadset.rules' '$RULES_DST'"
    echo "   sudo udevadm control --reload-rules && sudo udevadm trigger"
    echo ""
fi
exec "${HERE}/usr/bin/hyperx-ngenuity-open" "$@"
""", encoding="utf-8")
os.chmod(appdir / "AppRun", 0o755)

tool = REPO / "appimagetool-x86_64.AppImage"
extract = REPO / "squashfs-root"
if not extract.exists():
    if not tool.exists():
        subprocess.run(["wget", "-q", "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"], cwd=REPO, check=True)
        os.chmod(tool, 0o755)
    subprocess.run([str(tool), "--appimage-extract"], cwd=REPO, check=True)

out = REPO / "HyperX-NGENUITY-Open-x86_64.AppImage"
if out.exists(): out.unlink()
subprocess.run([str(extract / "AppRun"), str(appdir), str(out)], cwd=REPO, check=True)

size = out.stat().st_size // 1024 // 1024
print(f"\n{'='*60}")
print(f"✅ ГОТОВО: {out.name} ({size} MB)")
print(f"{'='*60}")
print("\nСледующие шаги:")
print("  1. Скачай AppImage через Explorer → ПКМ → Download")
print("  2. На Alt Linux: chmod +x AppImage && ./AppImage")
print("  3. Если каких-то .wav нет — зайди в src/audio/voice.rs,")
print("     закомментируй нужные BAT_XXX и ветки match, пересобери")