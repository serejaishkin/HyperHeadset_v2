# Tauri Updater: генерация ключей подписи

## 1. Сгенерировать пару ключей (один раз)
```powershell
cargo install tauri-cli --locked
cargo tauri signer generate -w ~/.tauri/hyperheadset.key
```
Вывод содержит:
- приватный ключ (пароль опционально) -> переменная `TAURI_SIGNING_PRIVATE_KEY`
- публичный ключ (base64) -> вставить в `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`

## 2. Настроить GitHub Secrets (repo Settings → Secrets → Actions)
| Secret | Значение |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | содержимое `hyperheadset.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | пароль (или пусто) |

## 3. Заменить pubkey в tauri.conf.json
```json
"updater": {
  "pubkey": "<публичный ключ из шага 1>",
  "endpoints": ["https://github.com/serejaishkin/HyperHeadset_v2/releases/latest/download/latest.json"]
}
```

## 4. Релиз с updater-артефактами
```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content ~/.tauri/hyperheadset.key -Raw)
npm run tauri build   # или cargo tauri build
```
Артефакты: `.exe`/`.msi` + `*.sig`. Загрузить их и `latest.json` в GitHub Release.

## Формат latest.json
```json
{
  "version": "0.2.11",
  "notes": "Release notes",
  "platforms": {
    "windows-x86_64": { "signature": "<содержимое .sig>", "url": "https://github.com/.../HyperHeadsetv2_0.2.11_x64-setup.exe" }
  }
}
```

## Проверка обновления из приложения (JS)
```js
const { check } = window.__TAURI__.updater; // при withGlobalTauri + плагин
// или @tauri-apps/plugin-updater в npm
const update = await check();
if (update) { await update.downloadAndInstall(); await relaunch(); }
```
