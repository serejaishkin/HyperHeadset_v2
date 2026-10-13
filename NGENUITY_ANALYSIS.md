# HyperX NGENUITY Analysis

## Overview

Analysis of the official HyperX NGENUITY application (version 5.38.0.0) to understand device communication protocols and potential improvements for HyperHeadsetv2.

## Application Structure

**Location:** `C:\Program Files\WindowsApps\33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt`

**Architecture:**
- UWP application (Universal Windows Platform)
- Native helper process: `NGenuity2Helper.exe`
- Background component for device communication
- App Services for inter-process communication

## Key Components

### 1. Airoha SDK (Primary HID Communication)

**Versions Used:**
- Airoha_SDK_2_10_2
- Airoha_SDK_3_3_1
- Airoha_SDK_3_3_4
- Airoha_SDK_3_5_2
- Airoha_SDK_5_3_0_2
- Airoha_SDK_5_5_0_2
- **Airoha_SDK_5_7_0_8** (Latest)

**Key Libraries:**
- `AirohaHidCoreLib.dll` - HID communication library
- `AirohaPeqLibrary.dll` - Parametric Equalizer (audio processing)

**Location:** `Assets/Native/Airoha_SDK_5_7_0_8/AirohaHidCoreDll.dll`

### 2. DTS:X Ultra Integration

**Library:** `DTSAPO.dll` - DTS Audio Processing Object

**Device Profiles (SSID):**
- `03F0018B` - Cloud II Wireless
- `03F001BF` - Unknown model
- `03F006BE` - Unknown model
- `03F0078A` - Unknown model
- `03F0089D` - Unknown model
- `03F008BE` - Unknown model
- `03F0098D` - Unknown model
- `03F00995` - Unknown model
- `03F00C9D` - Unknown model
- `03F00D93` - Unknown model

**Configuration Files:** `Assets/Native/AudioDTS/HyperX[SSID]/dts_apo4_hpxv2_oem_config_[SSID].xml`

**DTS Features:**
- DTS:X Ultra spatial audio
- Multiple content modes: Music, Movie, Voice
- Automatic content detection
- Speaker type: Headphone

### 3. OpenVINO (AI/ML)

**Purpose:** Face detection and AI features (likely for headset posture, ANC adaptation)

**Plugins:**
- CPU plugin
- GPU plugin
- Auto plugin
- Hetero plugin

**Libraries:**
- `openvino.dll`
- `openvino_intel_cpu_plugin.dll`
- `openvino_intel_gpu_plugin.dll`
- `opencv_world460.dll`

### 4. Firmware Update

**Tools:**
- `CodecFWUpdater.exe`
- `FWUpdater.dll`
- `HasteWirelessUpdater`
- `Haste2WirelessFWUpdate`
- `SynapticsUpdateWrapper.dll`

### 5. Additional Components

- `AitUVCExtApiWrapper.dll` - USB Video Class extension (for webcams)
- `Newtonsoft.Json.dll` - JSON parsing
- `Microsoft.Toolkit.Uwp.Notifications.dll` - Toast notifications

## Device Communication

### Current HyperHeadsetv2 Approach
- Uses `hidapi` crate for HID communication
- Direct USB HID reports
- Custom protocol implementation per device

### NGENUITY Approach
- Uses AirohaHidCoreLib (manufacturer SDK)
- Abstracted device communication
- Multiple SDK versions for different device generations

## Airoha SDK Integration Feasibility

### ⚠️ Cross-Platform Concerns

**Windows Only:**
- Airoha SDK libraries are Windows DLLs (.dll)
- No Linux or macOS versions found in the package
- Likely proprietary SDK from Airoha (now MediaTek)

**Licensing:**
- Proprietary license (not open source)
- Requires agreement with Airoha/MediaTek
- Cannot be redistributed in open-source project

### ✅ Potential Solutions

1. **Platform-Specific Integration:**
   - Use Airoha SDK on Windows via FFI (Foreign Function Interface)
   - Keep `hidapi` as fallback for Linux/macOS
   - This would improve Windows stability significantly

2. **Reverse Engineering:**
   - Analyze AirohaHidCoreLib.dll behavior
   - Implement equivalent functionality in Rust
   - Time-consuming but cross-platform

3. **Keep Current Approach:**
   - Continue using `hidapi` cross-platform
   - Improve protocol implementation based on observed behavior
   - Add better error handling and retry logic

## Recommendations

### 1. SSID Mapping

**Action:** Map device IDs in HyperHeadsetv2 to official NGENUITY SSIDs

**Current Device Files:**
- `src/devices/cloud_iii_wireless.rs`
- `src/devices/cloud_iii_s_wireless.rs`
- `src/devices/cloud_ii_wireless_dts.rs`
- `src/devices/cloud_ii_wireless.rs`
- `src/devices/cloud_ii_core_wireless.rs`
- `src/devices/cloud_alpha_wireless.rs`
- `src/devices/cloud_flight_wireless.rs`

**Task:** Extract device VID/PID from each implementation and map to SSID.

### 2. Windows HID Stability

**Current Issues:**
- Intermittent audio interruptions
- Possible conflicts with NGENUITY when both running

**Potential Causes:**
- Direct HID access vs SDK-managed access
- Lack of proper device sharing
- Incorrect HID report timing

**NGENUITY Approach:**
- SDK likely handles device sharing better
- Proper retry and reconnection logic
- Manufacturer-tested protocols

### 3. Protocol Improvements

**Observations from NGENUITY:**
- Multiple SDK versions suggest protocol evolution
- Device-specific implementations (like HyperHeadsetv2)
- DTS profiles for audio enhancement

**Implementation:**
- Study behavior of devices when both apps are running
- Capture HID traffic (via Wireshark USB capture or similar)
- Compare with known protocol implementations

### 4. Audio Enhancement

**DTS:X Ultra:**
- Windows-only DTS integration
- Requires proprietary DTSAPO.dll
- May not be feasible for open-source project

**Alternative:**
- Continue using Equalizer APO (Windows)
- EasyEffects (Linux)
- eqMac (macOS)
- These are user-configurable and open

## Cross-Platform Strategy

### Windows
- **Option A:** Keep `hidapi` (current, cross-platform)
- **Option B:** Integrate Airoha SDK via FFI (better stability, requires SDK license)
- **Recommendation:** Start with Option A, improve error handling. Consider Option B if SDK license is obtainable.

### Linux
- Continue using `hidapi`
- NGENUITY doesn't support Linux
- No Airoha SDK available
- Current approach is correct

### macOS
- Continue using `hidapi`
- NGENUITY doesn't support macOS
- No Airoha SDK available
- Current approach is correct

## Next Steps

1. **SSID Mapping:** Document device VID/PID and map to NGENUITY SSIDs
2. **HID Capture:** Capture HID traffic from NGENUITY for protocol analysis
3. **Protocol Comparison:** Compare HyperHeadsetv2 protocols with NGENUITY behavior
4. **Error Handling:** Improve Windows HID error handling based on SDK patterns
5. **Device Sharing:** Implement proper HID device sharing to avoid conflicts

## Notes

- **File Location:** This analysis is based on NGENUITY 5.38.0.0 installed on Windows
- **SDK License:** Airoha SDK is proprietary, not redistributable
- **Cross-Platform:** SDK is Windows-only, Linux/macOS must use `hidapi`
- **Stability:** SDK likely provides better Windows stability due to manufacturer support

## References

- **NGENUITY Path:** `C:\Program Files\WindowsApps\33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt`
- **Airoha SDK Location:** `Assets/Native/Airoha_SDK_5_7_0_8/`
- **DTS Configs:** `Assets/Native/AudioDTS/HyperX[SSID]/`

---

**Document Version:** 1.0
**Analysis Date:** 2026-10-13
**NGENUITY Version:** 5.38.0.0
