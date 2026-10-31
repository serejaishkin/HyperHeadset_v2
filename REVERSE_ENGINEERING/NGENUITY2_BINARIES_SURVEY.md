# NGenuity2: обзор остальных DLL/EXE (208 бинарников)

Метод: pefile (тип, экспорты, импорты), строки ASCII/UTF-16, dnfile+dncil для .NET, capstone + xref по строкам для нативных DLL. Основной протокол Cloud II/Alpha/Flight — см. `NGENUITY2_HEADSET_PROTOCOL.md`, Airoha — `AIROHA_RACE_PROTOCOL.md`.

## 1. Относящееся к гарнитурам

### Airoha SDK (HID-ядро, нативные x64)
| Версия | Файл | Экспортов | Отличия |
|---|---|---|---|
| 2.10.2 | AirohaHidCoreLib.dll | 52 | минимальный набор (батарея/ANC/EQ/DFU) |
| 3.3.1 / 3.3.4 | AirohaHidCoreLib.dll | 153 | + sidetone, voice prompts, game/chat mix |
| 3.5.2 | AirohaHidCoreLib.dll | 154 | |
| 3.10.1.5 | AirohaHidCoreLib.dll | 243 | + LEA dongle, мыши/клавиатуры AB1613, USB-MCU DFU |
| 5.3.0.2 / 5.5.0.2 | AirohaHidCoreDll.dll | 193 / 195 | другое имя DLL |
| 5.7.0.8 | AirohaHidCoreDll.dll | 246 | + `InitializeAirohaBleSDKEx`/`PairedBle` (BLE), `GetDongleRssi*`, `Set/GetDynamicLR*`, DFU из буфера |

Во всех версиях RACE-команды те же: GET feature `0x2C83`, SET feature `0x2C82`, батарея `0x0CD6`. Раскладка внутренних структур в 5.x другая (поля `+0xBC` вместо `+0xB4`), поэтому номера features автоматически не сверены; в 5.7 `GetAutoPowerOff`/`SetAutoPlayPause` показывают feature=6 — требует проверки по логу.

### NGenuity2GeneralHelper.exe (.NET, WinForms) — Ralphie и 3-в-1 донгл
Фоновый хелпер: определяет устройства по `HID#VID_03F0&PID_…`, показывает уведомления, шлёт служебные команды.
- Устройства (HID-пути): Ralphie `099C Col03/Col04`, `0EBC Col03/Col04`, `0AA0 MI_03 Col04`, `08B7 MI_03 Col03`; TIO-донгл `01BF MI_05 Col02 / MI_03 Col05`, `01C5 Col04/Col05`.
- Report ID `0x0C`, буфер 264 байта (Ralphie):
  - Get paired info: `0C 02 02 02 19`
  - Software reset: `0C 01 01 01 13`
  - «Magic moment» (событие `0C 02 02 03 16 01`) → ответ `0C 01 01 01 16 <0|1>`
  - Atlas: событие `0C 02 02 03 15`
- Блокировка Wi-Fi-канала (Ralphie raw + TIO): report `08`, `08 AE A1 07 <chan> 34`; ack `08 BE A1 07 <status>`. `chan` берётся из таблицы Wi-Fi-канал → значение донгла: 1→3, 2→4, 3→5, 4→6, 5→3, 6→4, 7→5, 8→6, 9→7, 10→8, 11→9, 12→10, 13→11, 14→11. Канал берётся из `netsh wlan show interfaces`.
- TIO-донгл: входящий отчёт `0xFB`, тип устройства в байте 10 (мышь/гарнитура/геймпад) → тост «Connected».

### StingerS/kinsapo_7p1_spk.dll — APO 7.1 (Synaptics APO)
Привязка PID: `0951:16DA`, `0951:1709`, `0951:170B`, `0951:1715` (Cloud Stinger S / Core-семейство, USB-аудио), также `03F0:0168`. Это аудио-драйвер, не протокол управления.

### AudioDTS (dtsapo64/dtscnt64/dtstech64, DtsHPXV2Apo4Service.exe) и DTSAPO.dll
DTS Headphone:X v2 APO. `DTSAPO.dll` — .NET COM-обёртка `DTS_APO_ControllerLib` (40 типов): `DTSAPOSystem`, `DTSEndpointEnumerator`, `IDTSEndpointInfo(2,3)` с `SetDefault/SetConfig/OpMode`, **`get_HeadsetConnected`**, `DTSAPOControlType/SampleRate`. Управляет только обработкой звука (реестр + COM), с HID-протоколом гарнитуры не связана.

## 2. Обновление прошивок (не нужны для управления)
| Файл | Что это |
|---|---|
| FWUpdater.dll (+Haste/Haste2/PulsefireHaste2CoreWireless) | Kingston-проект, экспорты `FWU_Update/FWU_GetFileInfo/FWU_Get_FW_ErrorCode`, `FWUpdater.hex` — мыши Pulsefire Haste |
| Snx.dll | SONiX ISP (`SnxISP_*`, SN8-прошивки) — мыши |
| SynapticsUpdateWrapper.dll, SynCPXFwUpdateApiCom, SynCapeFwupgradeApiCom | Synaptics/Conexant Cx2070x/Cx3198x (аудио-HID чипы), `UpdateFrimware`, `ReadDeviceFirmware` |
| CodecFWUpdater.exe (.NET) + RvcLib(64) | QuadCast 2 / 2S: `UpdateFW_USB`, `GetDevFirmwareInfo`, `WriteSN/ReadSN` |
| AirohaHidCoreLib `*Dfu*` | DFU Airoha-гарнитур |

## 3. Не относится к управлению гарнитурой
DuoCast (Realtek `Rtk*`, OpenVINO inference_engine, GNA) — микрофон DuoCast; `HPFace*`, `KeyStone`, `BkgSeg`, `opencv_world460`, `openvino*`, `tbb*` — HP Presence/камера; `AitUVCExt*` — UVC-расширения камеры; `cafapihp` — COM EQ-фильтр; `piSecurity/piTransforms` — защита/трансформации; `HPPVSDKWrapper`, `GoogleMeasurementProtocol`, `Newtonsoft.Json`, `Microsoft.*`, `netstandard`, `clrcompression`, `msvcp/vcruntime` — инфраструктура. `hidapitester.exe` — утилита hidapi (`--list`, `--send-output`, `--read-input`), удобна для ручной проверки пакетов.

## 4. Итог
Протоколов управления гарнитурой нового типа в оставшихся DLL нет: всё сводится к (а) Cloud II/Alpha/Flight (`21 BB …` / `06 FF BB …`), (б) Cloud III/Core (`0x66`), (в) Airoha RACE (`05 5A …`), (г) Ralphie/3-в-1 донгл (report `0x0C`, служебные команды). Недостающее — таблица PID→модель (в `NGenuity2.dll`, .NET Native) и HID-логи для проверки Airoha/Ralphie.
