# macOS HID Capture and Analysis Guide

## Overview

This guide provides step-by-step instructions for capturing and analyzing HID traffic from HyperX headsets on macOS to improve the HyperHeadsetv2 implementation.

## Prerequisites

### Hardware
- HyperX headset (Cloud II Wireless, Cloud III Wireless, etc.)
- USB receiver/dongle
- Mac with USB ports

### Software
- macOS 10.12+ (Sierra or later)
- Xcode Command Line Tools
- Homebrew (recommended)
- Root access (for low-level USB monitoring)

## Method 1: Using USB Prober (Basic Device Info)

### Step 1: Install Xcode Command Line Tools

```bash
xcode-select --install
```

### Step 2: Use USB Prober

USB Prober is included with Xcode Hardware IO Tools:

```bash
# Find USB Prober
open /Applications/Utilities/USB\ Prober.app
```

Or install via Homebrew:
```bash
brew install --cask usb-prober
```

### Step 3: Identify HyperX Device

In USB Prober:
1. Find your HyperX headset in the device tree
2. Note the vendor ID (VID) and product ID (PID)
3. Check the interfaces and endpoints
4. Look for HID interface (Interface Class 0x03)

**Expected VID/PID:**
- Vendor: 0x03F0 (HP) or 0x0951 (HyperX)
- Product: varies by model

### Step 4: Examine Interface Details

For the HID interface, note:
- Interface number
- Alternate setting number
- Endpoint addresses (IN/OUT)
- Max packet size

## Method 2: Using IORegistryExplorer (Advanced)

### Step 1: Install IORegistryExplorer

```bash
brew install --cask ioregistryexplorer
```

Or use the version included with Xcode Hardware IO Tools.

### Step 2: Browse IORegistry

In IORegistryExplorer:
1. Navigate to `IOService:/AppleUSBDevice`
2. Find your HyperX device
3. Examine properties like:
   - `idVendor`
   - `idProduct`
   - `kUSBVendorID`
   - `kUSBProductID`
   - `kUSBSerialNumberString`

### Step 3: Monitor HID Events

IORegistryExplorer can show HID events in real-time:
1. Select the device
2. Look for changes in properties when interacting with the headset
3. Note which properties change for each operation

## Method 3: Using IOHIDManager (Programmatic)

### Step 1: Create Swift Capture Script

Create `hid_capture.swift`:

```swift
import Foundation
import IOKit
import IOHIDLib

class HIDCapture {
    private var device: IOHIDDeviceRef?
    
    func findDevice() -> IOHIDDeviceRef? {
        let vendorID: Int = 0x03F0  // HP
        let productIDs: [Int] = [0x05B7, 0x0C9D, 0x06BE, 0x02CC, 0x1718, 0x018B]
        
        let matchingDict: [String: Any] = [
            kIOHIDVendorIDKey: vendorID,
            kIOHIDProductIDKey: productIDs
        ]
        
        let manager = IOHIDManagerCreate(kCFAllocatorDefault, matchingDict as CFDictionary)
        IOHIDManagerSetDeviceMatching(manager, matchingDict as CFDictionary)
        IOHIDManagerOpen(manager, 0)
        
        let devices = IOHIDManagerCopyDevices(manager)
        
        if let deviceList = devices as? [IOHIDDeviceRef], deviceList.count > 0 {
            return deviceList[0]
        }
        
        return nil
    }
    
    func captureHIDReports() {
        guard let device = findDevice() else {
            print("Device not found")
            return
        }
        
        self.device = device
        
        // Set input report callback
        let inputCallback: IOHIDReportCallback = { (context, result, sender, report, reportLength) in
            guard let report = report else { return }
            let data = Data(bytes: report, count: reportLength)
            print("RX:", data.map { String(format: "%02x", $0) }.joined(separator: " "))
        }
        
        IOHIDDeviceRegisterInputReportCallback(device, inputCallback, nil)
        
        print("Capturing HID reports... (Ctrl+C to stop)")
        
        RunLoop.current.run()
    }
}

let capture = HIDCapture()
capture.captureHIDReports()
```

### Step 2: Compile and Run

```bash
# Compile
swiftc -o hid_capture hid_capture.swift -framework IOKit -framework IOHIDLib

# Run
sudo ./hid_capture
```

## Method 4: Using IOHIDManager with Input Monitoring

### Step 1: Monitor Input Reports

For devices that send unsolicited input reports (like battery updates):

```swift
import Foundation
import IOKit
import IOHIDLib

class HIDMonitor {
    func startMonitoring() {
        let vendorID: Int = 0x03F0
        let productIDs: [Int] = [0x05B7, 0x0C9D]
        
        let matchingDict: [String: Any] = [
            kIOHIDVendorIDKey: vendorID,
            kIOHIDProductIDKey: productIDs
        ]
        
        let manager = IOHIDManagerCreate(kCFAllocatorDefault, matchingDict as CFDictionary)
        IOHIDManagerSetDeviceMatching(manager, matchingDict as CFDictionary)
        IOHIDManagerOpen(manager, 0)
        
        let devices = IOHIDManagerCopyDevices(manager)
        
        if let deviceList = devices as? [IOHIDDeviceRef] {
            for device in deviceList {
                print("Found device")
                
                // Register for value changes
                let callback: IOHIDValueCallback = { (context, result, sender, value) in
                    guard let element = value?.element else { return }
                    let usagePage = element.usagePage
                    let usage = element.usage
                    let intValue = value?.intValue ?? 0
                    
                    print("Value changed: page=\(usagePage) usage=\(usage) value=\(intValue)")
                }
                
                IOHIDDeviceRegisterInputValueCallback(device, callback, nil)
            }
        }
        
        print("Monitoring values... (Ctrl+C to stop)")
        RunLoop.current.run()
    }
}

let monitor = HIDMonitor()
monitor.startMonitoring()
```

## Method 5: Using Console.app for System Logs

### Step 1: Open Console.app

```bash
open /Applications/Utilities/Console.app
```

### Step 2: Filter for USB/HID Events

In Console.app:
1. In the search bar, type: `IOHID`
2. Or filter by: `USB`
3. Look for log entries related to your HyperX device

### Step 3. Perform Device Operations

While monitoring logs:
- Connect/disconnect the headset
- Toggle mute
- Adjust settings
- Note any HID-related log entries

## Method 6: Using tcpdump with USB Sniffing (Requires Hardware)

### Step 1: Check for USB Sniffing Support

Some Macs support USB sniffing with tcpdump. Check if your Mac has this capability:

```bash
# List available interfaces
tcpdump -D
```

Look for `usbmon` interfaces.

### Step 2: Capture USB Traffic

If USB sniffing is available:

```bash
# Start capture
sudo tcpdump -i usbmon1 -w hyperx_hid.pcap

# Perform device operations
# Stop capture with Ctrl+C
```

### Step 3: Analyze with Wireshark

```bash
# Install Wireshark
brew install --cask wireshark

# Open capture
open hyperx_hid.pcap
```

In Wireshark:
1. Filter for USB HID traffic
2. Look for interrupt transfers
3. Analyze packet structure

## Cross-Platform Comparison

### macOS vs Linux vs Windows

| Aspect | macOS | Linux | Windows |
|--------|-------|-------|---------|
| HID Framework | IOHIDManager | hidraw/hiddev | hidapi |
| Device Path | Registry path | /dev/hidraw* | USB device path |
| Report Format | Same | Same | Same |
| Timing | May differ | May differ | May differ |

### Protocol Compatibility

The HID protocol is generally consistent across platforms, but:
- **Report ID:** Usually 0x66 on all platforms
- **Command IDs:** Same numeric values
- **Packet structure:** Same byte layout
- **Response format:** Same magic bytes (0x06 0xFF 0xBB)

## Troubleshooting

### Device Not Detected

```bash
# Check if device is visible
system_profiler SPUSBDataType

# Look for HyperX devices
system_profiler SPUSBDataType | grep -i hyperx
```

### Permission Denied

```bash
# Grant Full Disk Access (macOS 10.15+)
# System Preferences → Security & Privacy → Full Disk Access
# Add Terminal or your application
```

### No HID Events Captured

1. Ensure device is connected
2. Verify correct VID/PID
3. Check if another application is using the device
4. Try different capture method

## Specific macOS Considerations

### App Sandbox

If running as a sandboxed app (like Tauri), you may need:
- HID device permission in entitlements
- Full Disk Access for device enumeration

### USB Cables

Some USB cables on Mac may have limitations:
- Use original cable if possible
- Avoid hubs for initial testing
- Check cable supports HID

### macOS Version Differences

- **macOS 10.15+**: Requires notarized applications
- **macOS 11+**: Universal binaries (Intel/ARM)
- **IOHIDManager**: Generally stable across versions

## Documenting Findings

### For Each Operation, Record:

1. **Operation name** (e.g., "Battery query")
2. **Request packet** (hex)
3. **Response packet** (hex)
4. **Command ID**
5. **Data payload meaning**
6. **Response data meaning**
7. **Timing characteristics**

### Example Entry

```markdown
### Battery Query (Cloud II Wireless)

**Device:** Cloud II Wireless DTS
**Platform:** macOS

**Request:**
```
66 02 10 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

**Response (50% battery):**
```
06 FF BB 0D 32 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

**Analysis:**
- Byte 0: 0x66 (report ID)
- Byte 1-3: 0x02 0x10 0x00 (header)
- Byte 4: 0x10 (GET_BATTERY command)
- Response byte 7: 0x32 (50% battery)
```

## Improving macOS Implementation

Based on captured data:

1. **Verify** packet structures match current implementation
2. **Identify** macOS-specific timing requirements
3. **Discover** undocumented features (EQ, LED, etc.)
4. **Optimize** IOHIDManager usage patterns
5. **Add** missing command implementations

## Tools Installation via Homebrew

```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install useful tools
brew install --cask usb-prober
brew install --cask ioregistryexplorer
brew install --cask wireshark
brew install swift-format
```

## References

- Apple HID Documentation: https://developer.apple.com/documentation/hid
- IOHIDManager Reference: https://developer.apple.com/documentation/iokit/iothidmanager
- HID Protocol Documentation: `HID_PROTOCOL.md`
- Device Implementations: `../src/devices/`
