# AirohaHidCoreLib.dll — разбор (SDK 3.10.1.5)

Нативная x64 DLL (C++, MSVC), 243 экспорта, ссылается на HID.DLL/SETUPAPI. Исходников/.NET-метаданных нет — разобрано дизассемблированием (capstone) по xref'ам на строки и RTTI-классам команд.

## Транспорт (HID)
- `HidController`/`UsbHidPhy`: открытие по VID:PID+serial, чтение входных отчётов, запись `WriteFile` (`HidD_SetOutputReport` в импортах).
- Пакет пишется как output-report **целиком**, дополненный нулями до `OutputReportByteLength`; первый байт `0x05` и есть Report ID.
- Входящий поток разбирается `HidTransport` (типы: RACE `0x05`, также ACL/HCI-event → HCI-часть для BT-сценариев).

## Кадр RACE
`05 | type | len16 LE | cmd16 LE | payload`, `len = 2 + len(payload)`.
Типы: `5A` запрос, `5B` ответ (со статусом), `5D` индикация/нотификация.
Relay (донгл → гарнитура): внешняя команда `0x0D01`, внутри — обычный RACE-пакет.

## Команды (по конструкторам классов `*Cmd`)
| Назначение | cmd | Тип ответа | Payload / ответ |
|---|---|---|---|
| Батарея `GetBatteryInfoCmd`/`RaceGetBattery` | 0x0CD6 | 5D | req: `role`; resp: `status, role, level%` |
| GET feature | 0x2C83 | 5B | req: `feature16`; resp: `status, feature16, value` |
| SET feature | 0x2C82 | 5B | req: `feature16, value...`; resp: `status, feature16` |
| MMI state | 0x2C80 / 0x138A… | 5B | — |
| ANC / PEQ | 0x0901 (get), 0x0E06 / 0x0900 (set) | 5B | — |
| Find-me | 0x2C01 | 5B | — |
| FW version | 0x1C07 | 5D | — |
| Agent channel | 0x0CD4 | 5B | — |
| Fota / log dump / NV | 0x1C00–0x1C1B, 0x1E04–0x1E07, 0x0A0C/0x0A0D | | служебные |
| Mouse/KB (AB1613) | 0x30xx | | Race1613* (мыши/клавиатуры HyperX) |
| LEA dongle | 0x2C90–0x2CB3, 0x2205–0x2222 | | сканирование/подключение LE Audio |

### Feature ID (константы из DLL)
| Feature | id | значение |
|---|---|---|
| Sidetone level | 6 | u16 |
| Sidetone state | 7 | u8 on/off |
| Advanced passthrough | 8 | u8 |
| Voice prompts | 9 | u8 on/off |
| Game/chat mix ratio | 0x0B | |
| Game mic volume | 0x0C | |
| Auto power-off, auto play/pause, touch | не константа | feature id берётся из таблицы устройства (поле менеджера `+0x4C`), определяется в рантайме по модели |

Mute микрофона в DLL отдельной командой не найден — на этих гарнитурах он аппаратный (кнопка), состояние приходит нотификацией.

## Что дальше
`src/airoha_hid.rs` содержит сборщики/парсеры кадров (с тестами). Чтобы подключить Airoha-гарнитуры в приложении, нужны их реальные VID:PID (привязка PID→модель лежит в `NGenuity2.dll`, .NET Native) и один HID-лог (`hid_capture.py`) для проверки role/feature auto-power-off.
