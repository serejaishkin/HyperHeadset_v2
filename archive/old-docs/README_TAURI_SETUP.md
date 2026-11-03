# Инструкция по установке Tauri в HyperHeadset_v2

## Шаг 1: Добавить [lib] в корневой Cargo.toml

Откройте `Cargo.toml` в корне проекта и добавьте в конец:

```toml
[lib]
path = "src/lib.rs"

[[bin]]
name = "hyperx-ngenuity-open"
path = "src/main.rs"
```

ВАЖНО: НЕ указывайте `name` в `[lib]` — Cargo сам сделает `hyperx_ngenuity_open` из `package.name`.

## Шаг 2: Скопировать файлы

Распакуйте этот архив в корень проекта:
- `src/lib.rs` → создаст библиотеку
- `src-tauri/` → Tauri backend
- `ui/` → HTML/CSS/JS фронтенд

## Шаг 3: Собрать

```bash
cd src-tauri
cargo tauri build
```

Готово! Бинарник будет в `src-tauri/target/release/`.
