# NGENUITY Reverse Engineering Roadmap

## Overview

This directory contains reverse engineering analysis of the official HyperX NGENUITY application to understand device protocols and enable cross-platform support.

## NGENUITY Version Analyzed

**Version:** 5.38.0.0
**Path:** `C:\Program Files\WindowsApps\33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt`
**Platform:** Windows UWP (Universal Windows Platform)

## Architecture Summary

### Core Components

| Component | Purpose | Platform | Notes |
|-----------|---------|----------|-------|
| **AirohaHidCoreLib.dll** | HID communication with headsets | Windows | Proprietary, needs reverse engineering |
| **AirohaPeqLibrary.dll** | Parametric Equalizer (audio processing) | Windows | Proprietary, needs alternative implementation |
| **DTSAPO.dll** | DTS:X Ultra spatial audio | Windows | Proprietary, alternative: Equalizer APO/EasyEffects |
| **NGenuity2Helper.exe** | Native helper process for elevated operations | Windows | Manages device drivers, firmware updates |
| **NGenuity2.dll** | Main application logic | Windows/UWP | .NET Native compiled |

## Reverse Engineering Priorities

### Priority 1: HID Communication Protocol (Critical)

**Target:** `AirohaHidCoreLib.dll`

**Why:** This is the core component that communicates with HyperX headsets. Understanding this protocol is essential for:
- Linux support (currently using `hidapi` with custom protocols)
- macOS support (currently using `hidapi` with custom protocols)
- Improving Windows stability and compatibility

**Approach:**
1. **DLL Analysis:** Use tools like:
   - IDA Pro / Ghidra for static analysis
   - dnSpy / ILSpy for .NET decompilation (if applicable)
   - API Monitor for runtime analysis
   - Wireshark with USB capture for HID traffic

2. **HID Traffic Capture:**
   - Install USBPcap on Windows
   - Capture HID traffic during:
     - Device connect/disconnect
     - Battery status queries
     - Mute toggle
     - Sidetone adjustment
     - Volume changes
     - Charging status

3. **Known Protocol Details:**
   - Current HyperHeadsetv2 uses custom protocol per device
   - Commands are 62-64 byte packets
   - Response format: `0x06 0xFF 0xBB CMD_ID DATA...`
   - Need to identify command IDs and data structures

**Expected Outcome:**
- Complete protocol documentation
- Command/response packet structure
- Device-specific variations
- Improved Linux/macOS implementation

### Priority 2: Equalizer/Audio Processing (High)

**Target:** `AirohaPeqLibrary.dll` and `DTSAPO.dll`

**Why:** Audio enhancement features:
- Parametric Equalizer
- DTS:X Ultra spatial audio
- Device-specific audio profiles

**Current Cross-Platform Alternatives:**
- **Windows:** Equalizer APO (already implemented)
- **Linux:** EasyEffects / PipeWire
- **macOS:** eqMac

**Approach:**
1. Analyze `AirohaPeqLibrary.dll` to understand EQ parameter format
2. Document EQ band configurations per device
3. Document DTS:X Ultra profiles (SSID-based)
4. Create cross-platform mapping

**Expected Outcome:**
- EQ parameter documentation
- Device-specific EQ presets
- Better cross-platform audio quality

### Priority 3: Firmware Update Mechanism (Medium)

**Target:** `CodecFWUpdater.exe`, `FWUpdater.dll`

**Why:** Support for firmware updates across platforms

**Approach:**
1. Analyze firmware update protocol
2. Document firmware file format
3. Implement cross-platform updater

**Expected Outcome:**
- Firmware update protocol documentation
- Linux/macOS firmware update support

### Priority 4: LED/Lighting Control (Low)

**Target:** Lighting settings in NGENUITY

**Why:** RGB customization for supported headsets

**Approach:**
1. Identify HID commands for LED control
2. Document color/brightness parameters
3. Implement cross-platform

**Expected Outcome:**
- LED control protocol documentation
- Cross-platform RGB support

## Platform-Specific Analysis

### Windows

**Status:** Fully supported via NGENUITY and HyperHeadsetv2

**Components to Reverse Engineer:**
- AirohaHidCoreLib.dll (for better stability)
- AirohaPeqLibrary.dll (for EQ)
- DTSAPO.dll (for DTS profiles)

**Tools:**
- API Monitor (for runtime analysis)
- Process Monitor (for file I/O)
- USBPcap (for HID traffic)
- Ghidra/IDA Pro (for static analysis)

### Linux

**Status:** Partially supported via `hidapi`

**Current Implementation:**
- Uses `hidapi` crate for HID communication
- Custom protocol implementations per device
- Equalizer via EasyEffects/PipeWire
- No DTS:X Ultra support (proprietary)

**Components Needed:**
- AirohaHidCoreLib equivalent (likely need to reimplement from scratch)
- Native EQ integration (PulseWire/JACK)
- Alternative to DTS:X Ultra (open-source spatial audio)

**Tools:**
- Wireshark with USB capture
- hidraw/hiddev analysis
- ALSA/PulseWire debugging

### macOS

**Status:** Partially supported via `hidapi`

**Current Implementation:**
- Uses `hidapi` crate for HID communication
- Custom protocol implementations per device
- Equalizer via eqMac
- No DTS:X Ultra support (proprietary)

**Components Needed:**
- AirohaHidCoreLib equivalent (likely need to reimplement from scratch)
- Native EQ integration (Audio Units framework)
- Alternative to DTS:X Ultra (open-source spatial audio)

**Tools:**
- USB Prober (for HID analysis)
- IORegistryExplorer (for device enumeration)
- Audio MIDI Setup (for audio routing)

## Known Protocol Details

### Packet Structure

**Common Format:**
```
Byte 0:       Report ID (0x66 for most devices)
Byte 1-3:     Header (varies by device)
Byte 4:       Command ID
Byte 5-N:     Data payload
```

**Response Format:**
```
Byte 0:       0x06
Byte 1:       0xFF
Byte 2:       0xBB
Byte 3:       Command ID (echo)
Byte 4-N:     Response data
```

### Known Command IDs

| Command ID | Purpose | Device |
|------------|---------|--------|
| 0x02 | Set auto shutdown | Cloud II Wireless |
| 0x03 | Set mute | Cloud II Wireless |
| 0x04 | Set silent mode | Cloud III Wireless |
| 0x0A | Mute response | Cloud II Wireless |
| 0x0C | Charging response | Cloud II Wireless |
| 0x0D | Battery response | Cloud II Wireless |
| 0x10 | Get battery | Common |
| 0x1E | Reset SIRK | Cloud III Wireless |
| 0x82 | Get SIRK | Cloud III Wireless |
| 0x84 | Get charging | Cloud II Wireless |
| 0x85 | Get mute | Cloud II Wireless |
| 0x86 | Get auto shutdown | Cloud II Wireless |
| 0x87 | Get sidetone | Common |
| 0x89 | Get voice prompts | Cloud II Wireless |

## Device-Specific Implementations

### Cloud II Wireless DTS
- **SSID:** 03F0018B, 03F00D93
- **VID:** 0x03F0 (HP)
- **PIDs:** 0x1718, 0x018B, 0x0D93, 0x0696
- **Protocol:** 20-byte packets
- **Features:** DTS:X Ultra support

### Cloud III Wireless
- **VID:** 0x03F0 (HP)
- **PIDs:** 0x05B7, 0x0c9d
- **Protocol:** 62-byte packets
- **Features:** Silent mode, SIRK

### Cloud III S Wireless
- **SSID:** 03F006BE
- **VID:** 0x03F0 (HP)
- **PIDs:** 0x06BE, 0x02CC
- **Protocol:** 62-byte packets
- **Features:** Enhanced microphone

### Cloud Alpha Wireless
- **SSID:** 03F0098D
- **VID:** 0x03F0 (HP)
- **PIDs:** 0x1743, 0x1765, 0x098D
- **Protocol:** 64-byte packets
- **Features:** 7.1 surround

### Cloud II Core Wireless
- **SSID:** 03F00995
- **VID:** 0x03F0 (HP)
- **PIDs:** 0x069F, 0x0995
- **Protocol:** 64-byte packets
- **Features:** Lightweight design

### Cloud Flight Wireless
- **VID:** 0x03F0 (HP), 0x0951 (HYPERX)
- **PIDs:** 0x0e90, 0x1749, 0x16c4
- **Protocol:** 64-byte packets
- **Features:** Wireless gaming headset

## Reverse Engineering Tools

### Windows
- **API Monitor:** Monitor API calls (HID, file I/O)
- **Process Monitor:** File system and registry activity
- **USBPcap:** USB traffic capture
- **Wireshark:** Network and USB packet analysis
- **Ghidra:** Free/open-source disassembler
- **IDA Pro:** Commercial disassembler
- **dnSpy:** .NET decompiler
- **ILSpy:** .NET decompiler
- **HxD:** Hex editor

### Linux
- **Wireshark:** USB capture with USBPcap-compatible devices
- **usbmon:** Linux USB monitoring
- **hexdump:** Binary analysis
- **strace:** System call tracing
- **ltrace:** Library call tracing
- **Ghidra:** Disassembler

### macOS
- **USB Prober:** USB device enumeration
- **IORegistryExplorer:** I/O Kit registry browser
- **Hopper Disassembler:** macOS disassembler
- **Ghidra:** Disassembler
- **class-dump:** Objective-C runtime introspection

## Current Status

### Completed
- ✅ SSID mapping for device IDs
- ✅ Basic HID protocol implementation
- ✅ Battery status
- ✅ Mute toggle
- ✅ Sidetone control
- ✅ Charging status
- ✅ Auto shutdown

### In Progress
- 🔄 Long press detection (using sidetone change as indicator)
- 🔄 Discord mute integration
- 🔄 Fallback audio device switching

### Pending
- ⏳ Complete HID protocol reverse engineering
- ⏳ EQ parameter documentation
- ⏳ Firmware update protocol
- ⏳ LED control protocol
- ⏳ DTS:X Ultra profile analysis

## Documentation References

- **Main Analysis:** `../NGENUITY_ANALYSIS.md`
- **Device Implementations:** `../src/devices/`
- **HID Abstraction:** `../src/device.rs`
- **Protocol Examples:** `../src/bin/packet_tester.rs`

## Next Steps

1. **HID Traffic Capture:** Capture complete HID traffic for all device operations
2. **DLL Static Analysis:** Analyze AirohaHidCoreLib.dll with Ghidra
3. **Protocol Documentation:** Document complete command/response format
4. **Cross-Platform Implementation:** Improve Linux/macOS based on findings
5. **EQ Analysis:** Document EQ parameters and DTS profiles
6. **Testing:** Test with multiple HyperX headset models

## Contributing

When adding new findings to this directory:
1. Create a subdirectory for the specific component being analyzed
2. Document the tool used and the procedure
3. Include raw captures and decoded data
4. Add interpretations and hypotheses
5. Update this ROADMAP.md with findings

## License Note

All reverse engineering findings in this directory are for educational purposes and to enable open-source alternatives to proprietary software. The findings should not be used to circumvent DRM or violate terms of service.
