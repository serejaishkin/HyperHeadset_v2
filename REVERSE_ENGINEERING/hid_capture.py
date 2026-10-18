#!/usr/bin/env python3
"""
Cross-platform HID Capture Tool for HyperX Headsets

This script captures HID traffic from HyperX headsets on Linux and macOS.
It requires the pyusb library for USB communication.

Installation:
    pip install pyusb

Usage:
    python3 hid_capture.py

The script will:
1. Find HyperX devices (VID 0x03F0 or 0x0951)
2. Capture HID interrupt transfers
3. Display packets in hex format
4. Log packets to file for analysis
"""

import usb.core
import usb.util
import time
import sys
from datetime import datetime

# HyperX vendor IDs
VENDOR_HP = 0x03F0
VENDOR_HYPERX = 0x0951

# Known product IDs
PRODUCT_IDS = [
    0x05B7,  # Cloud III Wireless
    0x0C9D,  # Cloud III Wireless
    0x06BE,  # Cloud III S Wireless
    0x02CC,  # Cloud III S Wireless
    0x1718,  # Cloud II Wireless
    0x018B,  # Cloud II Wireless DTS
    0x0D93,  # Cloud II Wireless DTS
    0x0696,  # Cloud II Wireless DTS
    0x069F,  # Cloud II Core Wireless
    0x0995,  # Cloud II Core Wireless
    0x1743,  # Cloud Alpha Wireless
    0x1765,  # Cloud Alpha Wireless
    0x098D,  # Cloud Alpha Wireless
    0x0E90,  # Cloud Flight Wireless
    0x1749,  # Cloud Flight Wireless
    0x16C4,  # Cloud Flight Wireless
]

class HIDCapture:
    def __init__(self, log_file=None):
        self.device = None
        self.ep_in = None
        self.ep_out = None
        self.log_file = log_file
        self.running = False
        
    def find_device(self):
        """Find a HyperX device."""
        print("Searching for HyperX device...")
        
        for vendor_id in [VENDOR_HP, VENDOR_HYPERX]:
            for product_id in PRODUCT_IDS:
                try:
                    dev = usb.core.find(idVendor=vendor_id, idProduct=product_id)
                    if dev:
                        self.print_device_info(dev)
                        return dev
                except usb.core.USBError:
                    continue
        
        print("No HyperX device found")
        return None
    
    def print_device_info(self, dev):
        """Print device information."""
        print(f"Device found: {dev}")
        print(f"  Vendor ID: 0x{dev.idVendor:04x}")
        print(f"  Product ID: 0x{dev.idProduct:04x}")
        print(f"  Manufacturer: {usb.util.get_string(dev, dev.iManufacturer, 256)}")
        print(f"  Product: {usb.util.get_string(dev, dev.iProduct, 256)}")
        print(f"  Serial: {usb.util.get_string(dev, dev.iSerialNumber, 256)}")
        
        # Count configurations
        for cfg in dev:
            print(f"  Configuration {cfg.bConfigurationValue}")
            for intf in cfg:
                print(f"    Interface {intf.bInterfaceNumber} ({intf.bInterfaceClass:02x})")
                for ep in intf:
                    print(f"      Endpoint {ep.bEndpointAddress:02x} ({ep.bmAttributes:02x})")
    
    def find_hid_endpoints(self, dev):
        """Find HID interrupt endpoints."""
        print("\nLooking for HID endpoints...")
        
        for cfg in dev:
            for intf in cfg:
                # Check if this is a HID interface (class 0x03)
                if intf.bInterfaceClass == 0x03:
                    print(f"Found HID interface {intf.bInterfaceNumber}")
                    
                    for ep in intf:
                        # Look for interrupt endpoints (type 0x03)
                        if ep.bmAttributes & 0x03 == 0x03:
                            if ep.bEndpointAddress & 0x80:  # IN endpoint
                                self.ep_in = ep
                                print(f"  IN endpoint: 0x{ep.bEndpointAddress:02x}")
                                print(f"    Max packet size: {ep.wMaxPacketSize}")
                            else:  # OUT endpoint
                                self.ep_out = ep
                                print(f"  OUT endpoint: 0x{ep.bEndpointAddress:02x}")
                                print(f"    Max packet size: {ep.wMaxPacketSize}")
        
        if self.ep_in:
            print("Successfully found HID IN endpoint")
        else:
            print("Warning: No HID IN endpoint found")
        
        if self.ep_out:
            print("Successfully found HID OUT endpoint")
        else:
            print("Warning: No HID OUT endpoint found")
    
    def capture_interrupts(self):
        """Capture HID interrupt transfers."""
        if not self.ep_in:
            print("No IN endpoint available for capture")
            return
        
        print("\nCapturing HID interrupt transfers...")
        print("Press Ctrl+C to stop")
        print("-" * 60)
        
        try:
            # Detach kernel driver if using Linux
            try:
                if sys.platform.startswith('linux'):
                    self.device.detach_kernel_driver(0)
                    print("Detached kernel driver")
            except usb.core.USBError as e:
                print(f"Warning: Could not detach kernel driver: {e}")
            
            # Set configuration
            self.device.set_configuration()
            
            # Claim interface
            cfg = self.device.get_active_configuration()
            for intf in cfg:
                if intf.bInterfaceClass == 0x03:
                    usb.util.claim_interface(self.device, intf)
                    print(f"Claimed HID interface {intf.bInterfaceNumber}")
                    break
            
            self.running = True
            
            while self.running:
                try:
                    # Read with timeout
                    data = self.ep_in.read(self.ep_in.wMaxPacketSize, timeout=1000)
                    if data:
                        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")
                        hex_str = " ".join(f"{b:02x}" for b in data)
                        print(f"[{timestamp}] RX ({len(data)} bytes): {hex_str}")
                        
                        # Log to file if specified
                        if self.log_file:
                            with open(self.log_file, 'a') as f:
                                f.write(f"{timestamp},RX,{len(data)},{hex_str}\n")
                    else:
                        # Timeout is expected
                        pass
                except usb.core.USBError as e:
                    if e.errno == 110:  # Timeout
                        pass
                    else:
                        print(f"USB Error: {e}")
                        break
                except KeyboardInterrupt:
                    print("\nCapture stopped by user")
                    self.running = False
                    
        finally:
            # Release interface
            try:
                usb.util.release_interface(self.device)
                print("Released interface")
            except:
                pass
            
            # Reattach kernel driver if on Linux
            try:
                if sys.platform.startswith('linux'):
                    self.device.attach_kernel_driver(0)
                    print("Reattached kernel driver")
            except:
                pass
    
    def print_help(self):
        """Print usage information."""
        print("HyperX HID Capture Tool")
        print("=" * 60)
        print("\nThis tool captures HID traffic from HyperX headsets.")
        print("\nUsage:")
        print("  python3 hid_capture.py")
        print("  python3 hid_capture.py --log capture.log")
        print("\nOptions:")
        print("  --log FILE    Log packets to file (CSV format)")
        print("  --help        Show this help message")
        print("\nNotes:")
        print("  - Requires pyusb: pip install pyusb")
        print("  - May require root/sudo access")
        print("  - Works on Linux and macOS")
        print("  - Captures interrupt transfers from headset")
        print("\nCommands to try while capturing:")
        print("  - Connect/disconnect headset")
        print("  - Toggle mute button")
        print("  - Adjust volume")
        print("  - Enable/disable sidetone")
        print("  - Check battery status")
        print("  - Put on/take off charging")

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='Capture HID traffic from HyperX headsets')
    parser.add_argument('--log', help='Log packets to file (CSV format)')
    parser.add_argument('--help', action='store_true', help='Show help message')
    
    args = parser.parse_args()
    
    if args.help:
        capture = HIDCapture()
        capture.print_help()
        return
    
    capture = HIDCapture(log_file=args.log)
    
    try:
        capture.device = capture.find_device()
        if capture.device:
            capture.find_hid_endpoints(capture.device)
            capture.capture_interrupts()
    except Exception as e:
        print(f"Error: {e}")
    finally:
        if capture.device:
            usb.util.dispose_resources(capture.device)
            print("Device resources released")

if __name__ == "__main__":
    main()
