# Фикс duplicate [lib]

## Проблема
В корневом Cargo.toml несколько раз добавлены секции [lib] и [[bin]] через `cat >>`.

## Решение

### Вариант А: заменить Cargo.toml целиком (рекомендуется)

1. Скопируйте `Cargo.toml` из этого архива в корень проекта (с заменой)
2. Скопируйте `src-tauri/Cargo.toml` из этого архива в `src-tauri/` (с заменой)
3. Проверьте, что в `src/config.rs` есть `#[derive(Default)]` перед `pub struct VoiceConfig`
4. Соберите:

```bash
cd ~/Documents/GitHub/HyperHeadset_v2/src-tauri
cargo tauri build
```

### Вариант Б: почистить вручную

Откройте `HyperHeadset_v2/Cargo.toml` и удалите ВСЕ блоки `[lib]` и `[[bin]]`, оставив только один `[lib]` и один `[[bin]]` в самом конце файла.

### Важно
- В `[lib]` НЕ должно быть строки `name = "..."` — только `path = "src/lib.rs"`
- В `src-tauri/Cargo.toml` обязательно должна быть секция `[workspace]`
