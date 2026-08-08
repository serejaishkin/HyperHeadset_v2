#!/usr/bin/env python3
"""
Сборка релиза в Codespaces:
- Linux AppImage (со звуком, если ассеты есть)
- macOS .app bundle (placeholder, нужен локальный бинарник под mac)
"""
import subprocess, sys, os, re, shutil
from pathlib import Path

REPO = Path.cwd()

def run(cmd, check=True):
    print(f"\n$ {cmd}")
    r = subprocess.run(cmd, shell=True)
    if check and r.returncode != 0:
        print(f"❌ FAIL: {cmd}")
        sys.exit(1)
    return r.returncode

def check_voice_assets():
    """Проверяем, что все include_bytes! в voice.rs имеют файлы."""
    voice_rs = REPO / "src/audio/voice.rs"
    if not voice_rs.exists():
        return [], []
    
    text = voice_rs.read_text()
    matches = re.findall(r'include_bytes!\s*\(\s*"([^"]+)"\s*\)', text)
    missing = []
    present = []
    for m in matches:
        p = (REPO / "src/audio" / m).resolve()
        if p.exists():
            present.append(p.relative_to(REPO))
        else:
            missing.append(p.relative_to(REPO))
    return present, missing

def build_linux_appimage(voice=True):
    print("\n" + "="*60)
    print("🐧 Сборка Linux AppImage")
    print("="*60)
    
    # Зависимости
    run("sudo apt-get update -qq")
    run("sudo apt-get install -y -qq libdbus-1-dev libhidapi-dev libusb-1.0-0-dev "
        "libudev-dev libasound2-dev pkg-config imagemagick desktop-file-utils")
    
    # Сборка
    if voice:
        run("cargo build --release")
    else:
        run("cargo build --release --no-default-features")
    
    # AppDir
    appdir = REPO / "HyperX-NGENUITY-Open.AppDir"
    if appdir.exists():
        shutil.rmtree(appdir)
    (appdir / "usr/bin").mkdir(parents=True)
    (appdir / "usr/share").mkdir(parents=True)
    (appdir / "usr/share/udev").mkdir(parents=True)
    
    shutil.copy(REPO / "target/release/hyperx-ngenuity-open", appdir / "usr/bin/")
    if (REPO / "lang").exists():
        shutil.copytree(REPO / "lang", appdir / "usr/share/lang", dirs_exist_ok=True)
    shutil.copy(REPO / "99-HyperHeadset.rules", appdir / "usr/share/udev/")
    
    # Иконка
    icon_src = REPO / "assets/icon.png"
    if not icon_src.exists():
        run("convert -size 256x256 xc:red " + str(appdir / "hyperx-ngenuity-open.png"))
    else:
        shutil.copy(icon_src, appdir / "hyperx-ngenuity-open.png")
    
    # Desktop
    (appdir / "hyperx-ngenuity-open.desktop").write_text("""[Desktop Entry]
Name=HyperX NGENUITY Open
Exec=hyperx-ngenuity-open
Type=Application
Icon=hyperx-ngenuity-open
Categories=AudioVideo;Audio;
""", encoding="utf-8")
    
    # AppRun
    (appdir / "AppRun").write_text("""#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"

RULES_SRC="${HERE}/usr/share/udev/99-HyperHeadset.rules"
RULES_DST="/etc/udev/rules.d/99-HyperHeadset.rules"

if [ ! -f "$RULES_DST" ]; then
    echo ""
    echo "⚠️  Нужны права на USB-гарнитуру. Выполните ОДИН раз:"
    echo "   sudo cp '$RULES_SRC' '$RULES_DST'"
    echo "   sudo udevadm control --reload-rules && sudo udevadm trigger"
    echo "   Или запустите: ./HyperX-NGENUITY-Open-x86_64.AppImage --install-udev-rules"
    echo ""
    if [ "$1" = "--install-udev-rules" ]; then
        if command -v pkexec >/dev/null 2>&1; then
            pkexec cp "$RULES_SRC" "$RULES_DST" && pkexec udevadm control --reload-rules && pkexec udevadm trigger && echo "✅ Rules установлены!" && exit 0
        elif command -v sudo >/dev/null 2>&1; then
            sudo cp "$RULES_SRC" "$RULES_DST" && sudo udevadm control --reload-rules && sudo udevadm trigger && echo "✅ Rules установлены!" && exit 0
        else
            echo "❌ Нет pkexec/sudo" && exit 1
        fi
    fi
fi
exec "${HERE}/usr/bin/hyperx-ngenuity-open" "$@"
""", encoding="utf-8")
    os.chmod(appdir / "AppRun", 0o755)
    
    # appimagetool — распаковываем, т.к. FUSE недоступен в Codespaces
    tool = REPO / "appimagetool-x86_64.AppImage"
    if not tool.exists():
        run("wget -q https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage")
        os.chmod(tool, 0o755)
    
    extract_dir = REPO / "squashfs-root"
    if not (extract_dir / "AppRun").exists():
        run(f"{tool} --appimage-extract")
    
    out = REPO / "HyperX-NGENUITY-Open-x86_64.AppImage"
    if out.exists():
        out.unlink()
    
    # Запускаем распакованный appimagetool
    run(f"{extract_dir}/AppRun {appdir} {out}")
    print(f"\n✅ Linux AppImage: {out} ({out.stat().st_size // 1024 // 1024} MB)")

def build_macos_bundle():
    print("\n" + "="*60)
    print("🍎 Подготовка macOS .app bundle (placeholder)")
    print("="*60)
    
    app_name = "HyperX NGENUITY Open.app"
    app_path = REPO / app_name
    if app_path.exists():
        shutil.rmtree(app_path)
    
    (app_path / "Contents/MacOS").mkdir(parents=True)
    (app_path / "Contents/Resources").mkdir(parents=True)
    
    # Placeholder: копируем Linux-бинарник
    bin_placeholder = REPO / "target/release/hyperx-ngenuity-open"
    if bin_placeholder.exists():
        shutil.copy(bin_placeholder, app_path / "Contents/MacOS/hyperx-ngenuity-open")
        os.chmod(app_path / "Contents/MacOS/hyperx-ngenuity-open", 0o755)
    
    # README
    (app_path / "Contents/MacOS/README.txt").write_text("""macOS Placeholder
=================
Этот бинарник собран под Linux. Для macOS:
1. Склонируйте репо на Mac
2. cargo build --release
3. Замените этот файл на:
   target/release/hyperx-ngenuity-open
""", encoding="utf-8")
    
    # Lang
    if (REPO / "lang").exists():
        shutil.copytree(REPO / "lang", app_path / "Contents/Resources/lang", dirs_exist_ok=True)
    
    # Иконка
    icon_src = REPO / "assets/icon.icns"
    if icon_src.exists():
        shutil.copy(icon_src, app_path / "Contents/Resources/AppIcon.icns")
    else:
        png = REPO / "assets/icon.png"
        if png.exists():
            try:
                from PIL import Image
            except ImportError:
                run("pip install Pillow -q", check=False)
                from PIL import Image
            img = Image.open(png).convert("RGBA")
            iconset = REPO / "icon.iconset"
            iconset.mkdir(exist_ok=True)
            for s in [16, 32, 128, 256, 512]:
                img.resize((s, s), Image.LANCZOS).save(iconset / f"icon_{s}x{s}.png")
                img.resize((s*2, s*2), Image.LANCZOS).save(iconset / f"icon_{s}x{s}@2x.png")
            r = subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(app_path / "Contents/Resources/AppIcon.icns")])
            if r.returncode != 0:
                img.resize((512, 512), Image.LANCZOS).save(app_path / "Contents/Resources/AppIcon.png")
                print("⚠️  iconutil не найден, использован 512×512 PNG")
            shutil.rmtree(iconset, ignore_errors=True)
    
    # Info.plist
    (app_path / "Contents/Info.plist").write_text("""<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>hyperx-ngenuity-open</string>
    <key>CFBundleIdentifier</key>
    <string>com.serezaiskin.hyperx-ngenuity-open</string>
    <key>CFBundleName</key>
    <string>HyperX NGENUITY Open</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSUIElement</key>
    <true/>
</dict>
</plist>
""", encoding="utf-8")
    
    # Упаковываем в zip
    zip_out = REPO / "HyperX-NGENUITY-Open-macOS-app.zip"
    if zip_out.exists():
        zip_out.unlink()
    run(f"zip -r {zip_out} '{app_name}'")
    print(f"\n✅ macOS bundle: {zip_out} ({zip_out.stat().st_size // 1024} KB)")
    print("   ⚠️  Внутри Linux-placeholder. Замените бинарник на macOS-сборку.")

def main():
    print("🔧 HyperHeadsetv2 — сборка релиза (Codespaces)")
    
    present, missing = check_voice_assets()
    print(f"\n🎤 Voice assets: найдено {len(present)}, отсутствует {len(missing)}")
    if missing:
        for m in missing:
            print(f"   ❌ {m}")
        print("\nСобрать БЕЗ голоса? [Y/n]: ", end="")
        ans = input().strip().lower()
        voice = (ans == "" or ans == "y" or ans == "yes")
        if voice:
            print("Переключаюсь на сборку без embedded-voice...")
            voice = False
    else:
        voice = True
        print("   ✅ Все voice-файлы на месте, собираем со звуком")
    
    build_linux_appimage(voice=voice)
    build_macos_bundle()
    
    print("\n" + "="*60)
    print("📦 ГОТОВЫЕ ФАЙЛЫ:")
    for f in ["HyperX-NGENUITY-Open-x86_64.AppImage", "HyperX-NGENUITY-Open-macOS-app.zip"]:
        p = REPO / f
        if p.exists():
            size = p.stat().st_size
            unit = "MB" if size > 1024*1024 else "KB"
            size = size // 1024 // 1024 if size > 1024*1024 else size // 1024
            print(f"   {f} ({size} {unit})")
    print("="*60)
    print("\nСледующие шаги:")
    print("  1. Скачай AppImage — запусти на Alt Linux")
    print("  2. Для macOS: распакуй zip, замени бинарник в .app/Contents/MacOS/")
    print("     на собранный локально через 'cargo build --release'")
    print("  3. Или запусти CI: git tag v0.2.10 && git push origin v0.2.10")

if __name__ == "__main__":
    main()