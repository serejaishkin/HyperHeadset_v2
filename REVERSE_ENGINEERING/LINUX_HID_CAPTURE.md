# Linux HID Capture and Analysis Guide

## Overview

This guide provides step-by-step instructions for capturing and analyzing HID traffic from HyperX headsets on Linux to improve the HyperHeadsetv2 implementation.

## Prerequisites

### Hardware
- HyperX headset (Cloud II Wireless, Cloud III Wireless, etc.)
- USB receiver/dongle
- Linux machine with USB ports

### Software
- Linux distribution (Ubuntu, Fedora, Arch, etc.)
- Root access (for USB monitoring)
- Wireshark
- `usbmon` kernel module
- `hidraw` tools
- `hexdump`
- `strace`

## Method 1: Using Wireshark with USBPcap

### Step 1: Install Wireshark

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install wireshark
```

**Fedora/RHEL:**
```bash
sudo dnf install wireshark
```

**Arch:**
```bash
sudo pacman -S wireshark-cli wireshark-qt
```

### Step 2: Add User to wireshark Group

```bash
sudo usermod -a -G wireshark $USER
```

**Logout and login again for changes to take effect.**

### Step 3: Enable USB Capture

Wireshark on Linux requires USBPcap for USB monitoring. Check if it's available:

```bash
# Check if USBPcap is available
ls /dev/usbmon*
```

If USBPcap is not available, enable usbmon:

```bash
# Load usbmon module
sudo modprobe usbmon

# Check available USB buses
ls /sys/kernel/debug/usb/usb_mon/
```

### Step 4: Identify USB Bus Number

Find your HyperX headset USB bus:

```bash
# List all USB devices
lsusb

# Look for HyperX devices (vendor ID 03F0 or 0951)
# Example output:
# Bus 001 Device 005: ID 03f0:05b7 HP, Inc.
```

Note the bus number (e.g., Bus 001).

### Step 5: Start USB Capture

```bash
# Start Wireshark
sudo wireshark
```

In Wireshark:
1. Go to Capture → Options
2. Add new capture filter
3. Select "usbmon" interface (e.g., usbmon1 for Bus 001)
4. Start capture

### Step 6: Perform Device Operations

While capturing, perform operations on the headset:
- Connect/disconnect the headset
- Toggle mute
- Adjust volume
- Enable/disable sidetone
- Check battery status
- Change charging state

### Step 7: Stop and Analyze Capture

1. Stop capture in Wireshark
2. Apply filter: `usb.capdata`
3. Look for HID interrupt transfers (URB_INTERRUPT)
4. Export interesting packets

**Filter for HID traffic:**
```
usb.capdata and usb.transfer_type == 0x03
```

## Method 2: Using hidraw Direct Monitoring

### Step 1: Identify hidraw Device

```bash
# List hidraw devices
ls -l /dev/hidraw*

# Find HyperX device
# Check device details
udevadm info -a -n /dev/hidraw0
```

Look for vendor ID (idVendor) and product ID (idProduct):
- Vendor: 03f0 (HP) or 0951 (HyperX)
- Product: varies by model

### Step 2: Monitor hidraw with strace

```bash
# Find HyperHeadsetv2 process
ps aux | grep hyperx-ngenuity

# Monitor its HID interactions
sudo strace -p <PID> -e trace=read,write -s 256 -xx
```

This will show raw hex data being read/written to hidraw devices.

### Step 3: Manual hidraw Reading

Create a Python script to monitor hidraw:

```python
#!/usr/bin/env python3
import fcntl
import os

DEVICE = "/dev/hidraw0"  # Replace with your device

try:
    fd = os.open(DEVICE, os.O_RDWR)
    
    # Set non-blocking mode
    flags = fcntl.fcntl(fd, fcntl.F_GETFL)
    fcntl.fcntl(fd, fcntl.F_SETFL, flags | os.O_NONBLOCK)
    
    print(f"Monitoring {DEVICE}...")
    print("Press Ctrl+C to stop")
    
    while True:
        try:
            data = os.read(fd, 64)
            if data:
                print("RX:", " ".join(f"{b:02x}" for b in data))
        except BlockingIOError:
            pass
        except KeyboardInterrupt:
            break
    
    os.close(fd)
except Exception as e:
    print(f"Error: {e}")
```

## Method 3: Using libusb Direct Capture

### Step 1: Install Dependencies

```bash
# Ubuntu/Debian
sudo apt install libusb-1.0-0-dev python3-pip

# Fedora/RHEL
sudo dnf install libusb1-devel python3-pip

# Arch
sudo pacman -S libusb python-pip
```

### Step 2: Install Python Libraries

```bash
pip install pyusb
```

### Step 3: Create USB Capture Script

```python
#!/usr/bin/env python3
import usb.core
import time

# HyperX vendor ID
VENDOR_ID = 0x03f0  # HP
# Product IDs (common models)
PRODUCT_IDS = [0x05b7, 0x0c9d, 0x06be, 0x02cc, 0x1718, 0x018b]

def find_device():
    for pid in PRODUCT_IDS:
        try:
            dev = usb.core.find(idVendor=VENDOR_ID, idProduct=pid)
            if dev:
                print(f"Found device: VID={VENDOR_ID:04x} PID={pid:04x}")
                return dev
        except usb.core.USBError:
            continue
    return None

def capture_hid_interrupt(dev):
    # Find HID interface
    for cfg in dev:
        for intf in cfg:
            for ep in intf:
                if ep.bEndpointAddress & 0x80:  # IN endpoint
                    if ep.bmAttributes & 0x03 == 0x03:  # Interrupt
                        print(f"Found HID interrupt endpoint: {ep.bEndpointAddress:02x}")
                        return ep
    return None

print("Searching for HyperX device...")
dev = find_device()

if dev:
    print("Device found, starting capture...")
    try:
        dev.set_configuration()
        ep = capture_hid_interrupt(dev)
        
        if ep:
            print("Capturing HID traffic... (Ctrl+C to stop)")
            while True:
                try:
                    data = ep.read(ep.wMaxPacketSize, timeout=1000)
                    if data:
                        print("RX:", " ".join(f"{b:02x}" for b in data))
                except usb.core.USBError as e:
                    if e.errno == 110:  # Timeout
                        continue
                    raise
    except KeyboardInterrupt:
        print("\nCapture stopped")
    finally:
        usb.util.dispose_resources(dev)
else:
    print("No HyperX device found")
```

## Analyzing Captured Data

### Identify Request/Response Pairs

1. **Look for 0x66 as first byte** - This is the report ID for most HyperX devices
2. **Match command IDs** - The 4th byte in request should match the 4th byte in response
3. **Note packet lengths** - 20, 62, or 64 bytes depending on device

### Document Commands

For each operation, record:
- Request packet (hex)
- Response packet (hex)
- Command ID
- Data payload meaning
- Response data meaning

### Example Analysis

**Battery Query Request:**
```
66 02 10 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
^^ ^^ ^^ 
|  |  +-- Command ID (0x10 = GET_BATTERY)
|  +----- Header (0x02 0x10)
+-------- Report ID (0x66)
```

**Response (50% battery):**
```
06 FF BB 0D 32 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
^^ ^^ ^^  ^^
|  |  |  +-- Battery percentage (0x32 = 50%)
|  |  +----- Command ID (0x0D = BATTERY_RESPONSE)
|  +-------- Magic bytes (0x06 0xFF 0xBB)
+------------ Response header
```

## Cross-Platform Considerations

### Linux vs Windows HID

**Differences:**
- Windows: Uses hidapi with device paths
- Linux: Uses hidraw/hiddev with device nodes
- macOS: Uses IOHIDManager framework

**Protocol Compatibility:**
- The HID protocol is generally the same across platforms
- However, timing and packet ordering may differ
- Some devices may have platform-specific quirks

### Improving Linux Implementation

Based on captured data:

1. **Verify packet structures** match current implementation
2. **Identify missing commands** not yet implemented
3. **Discover undocumented features** (EQ, LED, etc.)
4. **Improve error handling** based on observed failure modes
5. **Optimize timing** based on successful response patterns

## Troubleshooting

### Device Not Found

```bash
# Check if device is visible
lsusb | grep -i hyperx

# Check hidraw devices
ls -l /dev/hidraw*

# Check permissions
sudo chmod 666 /dev/hidraw0
```

### Permission Denied

```bash
# Add user to appropriate groups
sudo usermod -a -G plugdev $USER
sudo usermod -a -G input $USER

# Or use sudo for testing
sudo python3 capture_script.py
```

### No USB Traffic Captured

1. Ensure device is connected
2. Verify correct USB bus number
3. Check if device is in use by another application
4. Try different capture method

## Next Steps

1. **Capture complete traffic** for all device operations
2. **Document unknown command IDs** discovered during capture
3. **Test with multiple device models** to identify variations
4. **Update HID_PROTOCOL.md** with findings
5. **Implement missing features** based on captured data

## References

- HID Protocol Documentation: `HID_PROTOCOL.md`
- Device Implementations: `../src/devices/`
- Official Linux HID Documentation: https://www.kernel.org/doc/html/latest/hid/hid-intro.html
- libusb Documentation: https://libusb.info/
