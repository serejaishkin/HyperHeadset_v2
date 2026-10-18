# HID Protocol Documentation

## Overview

This document contains the current understanding of HyperX headset HID protocols based on the existing HyperHeadsetv2 implementation.

## General Packet Structure

### Request Format

Most HyperX headsets use a similar packet structure:

```
Byte 0:       Report ID (0x66 for most devices)
Byte 1-3:     Header (varies by device family)
Byte 4:       Command ID
Byte 5-N:     Data payload (command-specific)
Padding:     Padded to specific packet length (20, 62, or 64 bytes)
```

### Response Format

All responses follow this pattern:

```
Byte 0:       0x06
Byte 1:       0xFF
Byte 2:       0xBB
Byte 3:       Command ID (echo of request)
Byte 4-N:     Response data
```

### Validation

A response is considered valid if:
- Length >= 5 bytes
- Byte 0 == 0x06
- Byte 1 == 0xFF
- Byte 2 == 0xBB
- Byte 3 == requested Command ID

## Known Command IDs

### Common Commands

| Command ID | Name | Purpose | Devices |
|------------|------|---------|---------|
| 0x02 | SET_AUTO_SHUTDOWN | Set auto-shutdown timer | Cloud II Wireless |
| 0x03 | SET_MUTE | Toggle microphone mute | Cloud II Wireless |
| 0x04 | SET_SILENT_MODE | Set silent mode | Cloud III Wireless |
| 0x0A | MUTE_RESPONSE | Mute status response | Cloud II Wireless |
| 0x0C | CHARGING_RESPONSE | Charging status response | Cloud II Wireless |
| 0x0D | BATTERY_RESPONSE | Battery level response | Cloud II Wireless |
| 0x10 | GET_BATTERY | Query battery level | Common |
| 0x1E | RESET_SIRK | Reset SIRK (Secure Identity Resolving Key) | Cloud III Wireless |
| 0x82 | GET_SIRK | Get SIRK | Cloud III Wireless |
| 0x84 | GET_CHARGING | Query charging status | Cloud II Wireless |
| 0x85 | GET_MUTE | Query mute status | Cloud II Wireless |
| 0x86 | GET_AUTO_SHUTDOWN | Query auto-shutdown timer | Cloud II Wireless |
| 0x87 | GET_SIDE_TONE | Query sidetone status | Common |
| 0x89 | GET_VOICE_PROMPTS | Query voice prompts | Cloud II Wireless |

## Device-Specific Implementations

### Cloud II Wireless DTS

**File:** `src/devices/cloud_ii_wireless_dts.rs`

**Packet Length:** 20 bytes

**Base Packet:**
```rust
const BASE_PACKET: [u8; 20] = [0x66, 0x02, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
```

**VID/PID:**
- Vendor: 0x03F0 (HP)
- Products: 0x1718, 0x018B, 0x0D93, 0x0696

**Known Commands:**
- Battery: Command 0x10, Response byte 7 = percentage
- Mute: Command 0x03, Response byte 4 = status (1 = muted)
- Charging: Command 0x84, Response byte 4 = status (1 = charging)
- Sidetone: Command 0x87, Response byte 4 = status (1 = enabled)
- Auto-shutdown: Command 0x86, Response byte 4 = minutes

### Cloud III Wireless

**File:** `src/devices/cloud_iii_wireless.rs`

**Packet Length:** 62 bytes

**Base Packet:**
```rust
const BASE_PACKET: [u8; 62] = {
    let mut packet = [0; 62];
    packet[0] = 102; // 0x66
    packet
};
```

**VID/PID:**
- Vendor: 0x03F0 (HP)
- Products: 0x05B7, 0x0c9d

**Unique Commands:**
- SIRK (Secure Identity Resolving Key): Commands 0x1E (reset), 0x82 (get)
- Silent Mode: Commands 0x84 (get), 0x04 (set)
- Charging: Command 0x8A, Response byte 5 = status

### Cloud III S Wireless

**File:** `src/devices/cloud_iii_s_wireless.rs`

**Packet Length:** 62 bytes

**VID/PID:**
- Vendor: 0x03F0 (HP)
- Products: 0x06BE, 0x02CC

**Features:**
- Enhanced microphone control
- Uses SIRK mechanism

### Cloud Alpha Wireless

**File:** `src/devices/cloud_alpha_wireless.rs`

**Packet Length:** 64 bytes

**VID/PID:**
- Vendor: 0x03F0 (HP)
- Products: 0x1743, 0x1765, 0x098D

**Features:**
- 7.1 surround sound
- Extended packet length

### Cloud II Core Wireless

**File:** `src/devices/cloud_ii_core_wireless.rs`

**Packet Length:** 64 bytes

**VID/PID:**
- Vendor: 0x03F0 (HP)
- Products: 0x069F, 0x0995

**Features:**
- Lightweight design
- Similar to Cloud Alpha protocol

### Cloud Flight Wireless

**File:** `src/devices/cloud_flight_wireless.rs`

**Packet Length:** 64 bytes

**VID/PID:**
- Vendor: 0x03F0 (HP), 0x0951 (HYPERX)
- Products: 0x0e90, 0x1749, 0x16c4

**Features:**
- Wireless gaming headset
- Multi-vendor support

## Protocol Examples

### Battery Query (Cloud II Wireless DTS)

**Request:**
```
66 02 10 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

**Response (at 50% battery):**
```
06 FF BB 0D 32 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
         ^^ Command ID
           ^^ Battery percentage (50 = 0x32)
```

### Mute Toggle (Cloud II Wireless DTS)

**Request (mute on):**
```
66 02 03 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
                           ^^ Data: 1 = mute on
```

**Response:**
```
06 FF BB 0A 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
         ^^ Command ID
           ^^ Status (1 = muted)
```

### Sidetone Query (Common)

**Request:**
```
66 02 87 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

**Response (sidetone on):**
```
06 FF BB 87 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
         ^^ Command ID
           ^^ Status (1 = enabled)
```

## Implementation Details

### Building Packets

**Function:** `build_packet(cmd_id: u8, data: &[u8]) -> Vec<u8>`

**Process:**
1. Copy base packet template
2. Set command ID at byte 4
3. Copy data payload starting at byte 5
4. Return packet

### Sending Commands

**Windows Special Case:**
Some devices on Windows require `send_feature_report()` instead of `write()` when normal write fails with "Incorrect function" error.

**Retry Logic:**
- Write operations: 3 retries with 50ms delay
- Read operations: 2 retries with 50ms delay
- Detects transient errors (busy, access, timeout)

### Response Handling

**Timeout:** 2000ms for read operations

**Validation:**
- Check minimum length (5 bytes for basic, 8 bytes for battery)
- Check magic bytes (0x06 0xFF 0xBB)
- Check command ID match

## Unknown Protocol Aspects

### Command Discovery

The following aspects are not fully documented:
- Complete command ID range (0x00-0xFF)
- Data payload structure for each command
- Command sequences (e.g., multi-step operations)
- Device-specific variations in command behavior

### Status Bytes

Some responses have multiple status bytes beyond byte 4. Their meaning is unknown:
- Byte 5 and beyond in various responses
- Additional flags or parameters
- Error codes in response

### Advanced Features

Not yet reverse engineered:
- EQ parameter commands
- LED control commands
- Firmware update protocol
- Surround sound configuration
- ANC (Active Noise Cancellation) control

## Tool Recommendations for Further Analysis

### Windows
- **API Monitor:** Monitor HID API calls
- **USBPcap:** Capture USB traffic
- **Wireshark:** Analyze USB packets
- **Ghidra:** Static analysis of AirohaHidCoreLib.dll

### Linux
- **Wireshark:** USB capture with appropriate hardware
- **usbmon:** Kernel USB monitoring
- **hexdump:** Binary packet inspection
- **strace:** System call tracing

### macOS
- **USB Prober:** USB device enumeration
- **IORegistryExplorer:** Device properties
- **tcpdump:** With USB sniffing hardware
- **Class-dump:** Objective-C runtime analysis

## Next Steps for Complete Protocol Documentation

1. **HID Traffic Capture:** Capture complete USB traffic for all operations
2. **Command Mapping:** Document command ID to function mapping
3. **Parameter Discovery:** Identify data payload meanings
4. **Sequence Analysis:** Document multi-command sequences
5. **Device Variation:** Document differences between device models
6. **Validation:** Verify findings across multiple devices

## References

- Device implementations: `../src/devices/`
- HID abstraction: `../src/device.rs`
- Protocol tester: `../src/bin/packet_tester.rs`
- NGENUITY analysis: `../NGENUITY_ANALYSIS.md`
