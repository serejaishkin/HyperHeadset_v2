#!/usr/bin/env bash
# Builds the Tauri (HyperHeadsetv2) installer from scratch and puts it in ./release/.
# Run from Git Bash in the repo root:  ./build_tauri_installer.sh
set -euo pipefail
cd "$(dirname "$0")"

VER=$(grep -m1 '^version' src-tauri/Cargo.toml | cut -d'"' -f2)
echo "== Building HyperHeadsetv2 v$VER =="

# Never pick up an installer from an earlier build.
rm -rf src-tauri/target/release/bundle release
rm -f src-tauri/target/release/hyperx-ngenuity-open-tauri.exe
mkdir -p release

EXTRA=()
if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  # Updater artifacts need a signing key; without one the build would fail at the very end.
  EXTRA=(--config '{"bundle":{"createUpdaterArtifacts":false}}')
fi

if command -v cargo-tauri >/dev/null 2>&1 || cargo tauri --version >/dev/null 2>&1; then
  (cd src-tauri && cargo tauri build "${EXTRA[@]}")
else
  npx tauri build "${EXTRA[@]}"
fi

EXE=src-tauri/target/release/hyperx-ngenuity-open-tauri.exe
[ -f "$EXE" ] || { echo "!! $EXE not found - the build failed, see errors above"; exit 1; }
if ! grep -aq "Log started" "$EXE"; then
  echo "!! $EXE does not contain the current code (stale build)"; exit 1
fi

cp src-tauri/target/release/bundle/nsis/*.exe release/ 2>/dev/null || true
cp src-tauri/target/release/bundle/msi/*.msi release/ 2>/dev/null || true
cp "$EXE" "release/hyperx-ngenuity-open-tauri_v$VER.exe"

echo
echo "== Done. Files in ./release: =="
ls -l release
echo
echo "Installer must be named *${VER}*; the app title bar shows v$VER."
