# HyperX NGENUITY Open

Кроссплатформенная open-source альтернатива HyperX NGENUITY для Cloud II Wireless.

## Возможности

- 🔋 Уровень заряда в реальном времени
- 🎚️ Системный 10-полосный эквалайзер
- 🔊 Discord интеграция (keybind или Rich Presence)
- ⚙️ Настройки гарнитуры (sidetone, auto-shutdown, voice prompts)
- 🖥️ Системный трей

## Поддерживаемые платформы

| Платформа | Статус |
|-----------|--------|
| Windows   | ✅ Полная поддержка (Equalizer APO) |
| Linux     | ✅ Поддержка (EasyEffects / PipeWire) |
| macOS     | ⚠️ Базовая (eqMac) |

## Установка

```bash
git clone https://github.com/yourname/hyperx-ngenuity-open
cd hyperx-ngenuity-open
cargo build --release
```

## Использование

```bash
./target/release/hyperx-ngenuity-open
```

## Архитектура

Проект основан на [HyperHeadset](https://github.com/LennardKittner/HyperHeadset) (MIT) с добавлением GUI, системного EQ и Discord интеграции.

## Лицензия

MIT
