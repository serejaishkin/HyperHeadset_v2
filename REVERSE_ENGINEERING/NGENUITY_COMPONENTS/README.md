# NGENUITY Component Extraction and Analysis

## ⚠️ IMPORTANT LEGAL NOTICE

**DO NOT commit proprietary NGENUITY files to this repository.**

This directory is for:
- Documentation of file structure
- Analysis scripts
- Personal study notes
- Reverse engineering findings

**Proprietary files (DLL, EXE, etc.) should:**
- Never be committed to the repository
- Never be shared publicly
- Only be used for personal reverse engineering
- Respect HP Inc. copyright and licensing

## Directory Structure

```
REVERSE_ENGINEERING/
├── ROADMAP.md                    # Overall reverse engineering plan
├── HID_PROTOCOL.md               # Current HID protocol documentation
├── LINUX_HID_CAPTURE.md          # Linux HID capture guide
├── MACOS_HID_CAPTURE.md          # macOS HID capture guide
├── hid_capture.py               # Cross-platform HID capture tool
├── NGENUITY_COMPONENTS/          # For extracted components (NOT in git)
│   ├── README.md                # This file
│   ├── extraction_instructions.md  # How to extract components
│   ├── dll_analysis/             # DLL analysis findings
│   ├── protocol_analysis/        # Protocol analysis findings
│   └── notes/                   # Personal notes
└── ORIGINAL_APP/
    ├── README.md                # Original app structure documentation
    ├── components.md            # List of components and their purpose
    └── analysis_questions.md     # Questions to answer through analysis
```

## NGENUITY Installation Location

**Path:** `C:\Program Files\WindowsApps\33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt`

**Version:** 5.38.0.0

## Component List

### Core Components

| Component | Purpose | Size | Reverse Engineering Priority |
|-----------|---------|------|---------------------------|
| **NGenuity2.dll** | Main application logic | ~1MB | High |
| **NGenuity2Helper.exe** | Native helper process | ~500KB | Medium |
| **AirohaHidCoreLib.dll** | HID communication | ~200KB | **Critical** |
| **AirohaPeqLibrary.dll** | Parametric Equalizer | ~150KB | High |
| **DTSAPO.dll** | DTS:X Ultra spatial audio | ~2MB | Medium |
| **AirohaSDK_5_7_0_8/** | SDK versions | Various | **Critical** |

### SDK Versions

```
Assets/Native/
├── Airoha_SDK_2_10_2/
├── Airoha_SDK_3_3_1/
├── Airoha_SDK_3_3_4/
├── Airoha_SDK_3_5_2/
├── Airoha_SDK_5_3_0_2/
├── Airoha_SDK_5_5_0_2/
└── Airoha_SDK_5_7_0_8/
    └── AirohaHidCoreDll.dll
```

### Configuration Files

| File | Purpose | Format |
|------|---------|--------|
| `plugins.xml` | OpenVINO plugin configuration | XML |
| `AppxManifest.xml` | App package manifest | XML |
| `AppxBlockMap.xml` | App block map | XML |
| `NGenuity2.xr.xml` | XAML resource mapping | XML |

### DTS Profiles

Location: `Assets/Native/AudioDTS/HyperX[SSID]/`

| SSID | Device |
|------|--------|
| 03F0018B | Cloud II Wireless DTS |
| 03F001BF | Unknown model |
| 03F006BE | Cloud III S Wireless |
| 03F0078A | Unknown model |
| 03F0089D | Unknown model |
| 03F008BE | Unknown model |
| 03F0098D | Cloud Alpha Wireless |
| 03F00995 | Cloud II Core Wireless |
| 03F00C9D | Unknown model |
| 03F00D93 | Cloud II Wireless DTS |

## Extraction Instructions

### Method 1: Manual Copy (Recommended)

**For personal analysis only:**

1. Create extraction directory:
```bash
mkdir -p NGENUITY_COMPONENTS/extracted
```

2. Copy specific components (DO NOT commit to git):
```bash
# Copy only what you need for analysis
cp "C:/Program Files/WindowsApps/33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt/NGenuity2.dll" NGENUITY_COMPONENTS/extracted/
cp "C:/Program Files/WindowsApps/33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt/Assets/Native/Airoha_SDK_5_7_0_8/AirohaHidCoreDll.dll" NGENUITY_COMPONENTS/extracted/
cp "C:/Program Files/WindowsApps/33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt/Assets/Native/AirohaPeqLibrary.dll" NGENUITY_COMPONENTS/extracted/
```

3. Add to .gitignore:
```bash
echo "REVERSE_ENGINEERING/NGENUITY_COMPONENTS/extracted/" >> .gitignore
echo "REVERSE_ENGINEERING/NGENUITY_COMPONENTS/notes/" >> .gitignore
```

### Method 2: Automated Extraction Script

Create `REVERSE_ENGINEERING/extract_components.bat`:

```batch
@echo off
set SRC="C:\Program Files\WindowsApps\33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt"
set DST="NGENUITY_COMPONENTS\extracted"

echo Extracting NGENUITY components...
echo Source: %SRC%
echo Destination: %DST%

mkdir %DST% 2>nul

echo Copying main DLL...
copy "%SRC%\NGenuity2.dll" "%DST%\" 2>nul

echo Copying Airoha HID library...
copy "%SRC%\Assets\Native\Airoha_SDK_5_7_0_8\AirohaHidCoreDll.dll" "%DST%\" 2>nul

echo Copying Airoha PEQ library...
copy "%SRC%\Assets\Native\AirohaPeqLibrary.dll" "%DST%\" 2>nul

echo Copying DTS APO...
copy "%SRC%\Assets\Native\DTSAPO.dll" "%DST%\" 2>nul

echo Copying all SDK versions...
xcopy "%SRC%\Assets\Native\Airoha_SDK_*" "%DST%\sdk_versions\" /E /I /Y 2>nul

echo Copying DTS configs...
xcopy "%SRC%\Assets\Native\AudioDTS" "%DST%\dts_configs\" /E /I /Y 2>nul

echo Copying XML manifests...
copy "%SRC%\AppxManifest.xml" "%DST%\" 2>nul
copy "%SRC%\plugins.xml" "%DST%\" 2>nul

echo.
echo Extraction complete!
echo Files saved to: %DST%
echo.
echo IMPORTANT: DO NOT commit these files to git repository
```

### Method 3: PowerShell Script

Create `REVERSE_ENGINEERING/extract_components.ps1`:

```powershell
$Source = "C:\Program Files\WindowsApps\33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt"
$Destination = "NGENUITY_COMPONENTS\extracted"

Write-Host "Extracting NGENUITY components..." -ForegroundColor Cyan
Write-Host "Source: $Source" -ForegroundColor Gray
Write-Host "Destination: $Destination" -ForegroundColor Gray

New-Item -ItemType Directory -Path $Destination -Force | Out-Null

# Copy critical components
$FilesToCopy = @(
    "NGenuity2.dll"
    "Assets\Native\Airoha_SDK_5_7_0_8\AirohaHidCoreDll.dll"
    "Assets\Native\AirohaPeqLibrary.dll"
    "Assets\Native\DTSAPO.dll"
)

foreach ($File in $FilesToCopy) {
    $SrcPath = Join-Path $Source $File
    $DstPath = Join-Path $Destination (Split-Path $File -Leaf)
    $Dir = Split-Path $DstPath -Parent
    
    if (-not (Test-Path $Dir)) {
        New-Item -ItemType Directory -Path $Dir -Force | Out-Null
    }
    
    if (Test-Path $SrcPath) {
        Copy-Item $SrcPath $DstPath -Force
        Write-Host "Copied: $File" -ForegroundColor Green
    } else {
        Write-Host "Not found: $File" -ForegroundColor Yellow
    }
}

# Copy SDK versions
$SDKDir = Join-Path $Destination "sdk_versions"
New-Item -ItemType Directory -Path $SDKDir -Force | Out-Null

Get-ChildItem "$Source\Assets\Native\Airoha_SDK_*" | ForEach-Object {
    Copy-Item $_.FullName $SDKDir -Recurse -Force
    Write-Host "Copied SDK: $($_.Name)" -ForegroundColor Green
}

# Copy DTS configs
$DTSDir = Join-Path $Destination "dts_configs"
New-Item -ItemType Directory -Path $DTSDir -Force | Out-Null

Get-ChildItem "$Source\Assets\Native\AudioDTS" | ForEach-Object {
    Copy-Item $_.FullName $DTSDir -Recurse -Force
    Write-Host "Copied DTS config: $($_.Name)" -ForegroundColor Green
}

Write-Host "`nExtraction complete!" -ForegroundColor Cyan
Write-Host "Files saved to: $Destination" -ForegroundColor Gray
Write-Host "`nIMPORTANT: DO NOT commit these files to git repository" -ForegroundColor Red
```

## Analysis Tools Setup

### Ghidra Installation (Free/Open Source)

**Windows:**
1. Download from: https://ghidra-sre.org/
2. Extract and run `ghidraRun.bat`
3. Create new project for each DLL

**Linux:**
```bash
# Install via package manager
sudo apt install ghidra  # Ubuntu/Debian
sudo dnf install ghidra  # Fedora/RHEL
sudo pacman -S ghidra  # Arch
```

**macOS:**
```bash
brew install --cask ghidra
```

### IDA Pro (Commercial)

Available at: https://www.hex-rays.com/ida-pro/

Free version: IDA Free (non-commercial use only)

### dnSpy / ILSpy (.NET Decompilation)

**Windows:**
- dnSpy: https://github.com/dnSpy/dnSpy/releases
- ILSpy: Included with ILSpy package

## Analysis Priorities

### 1. AirohaHidCoreLib.dll (Critical)

**Goal:** Understand complete HID protocol

**Approach:**
1. Open in Ghidra
2. Find HID API functions:
   - `HidD_GetAttributes`
   - `HidD_SetFeature`
   - `HidD_GetFeature`
   - `HidD_Write`
   - `HidD_Read`
3. Look for command IDs (0x02, 0x03, 0x10, etc.)
4. Document packet construction
5. Find device-specific variations

**Questions to Answer:**
- How does it construct the base packet?
- What is the command ID mapping?
- How does it handle different device models?
- What are the timeout values?
- How does it handle errors/retries?

### 2. AirohaPeqLibrary.dll (High)

**Goal:** Understand EQ parameter format

**Approach:**
1. Open in Ghidra
2. Find EQ-related functions
3. Document frequency band structures
4. Document preset formats
5. Map to Equalizer APO format

**Questions to Answer:**
- What are the EQ band frequencies?
- What are the gain ranges?
- How are presets stored?
- Can we map to Equalizer APO config?

### 3. DTSAPO.dll (Medium)

**Goal:** Understand DTS:X Ultra profiles

**Approach:**
1. Analyze DTS config XML files
2. Document profile parameters
3. Map to device SSIDs
4. Check if open-source alternatives exist

**Questions to Answer:**
- What are the DTS profile parameters?
- Can we recreate in open-source EQ?
- What are the spatial audio settings?

## Documentation Template

For each analyzed component, create a markdown file:

```markdown
# [Component Name] Analysis

## Component Information

- **File:** [filename]
- **Size:** [size]
- **Purpose:** [purpose]
- **Architecture:** .NET Native / C++ / etc.

## Findings

### Key Functions

| Function | Address | Purpose |
|----------|---------|---------|
| [name] | [addr] | [description] |

### Protocol Details

[Document protocol structure]

### Unmapped Functions

[List functions with unknown purpose]

## Questions

[Open questions for further analysis]

## Notes

[Personal analysis notes]
```

## Git Configuration

Add to `.gitignore`:

```
# Reverse engineering - DO NOT commit proprietary files
REVERSE_ENGINEERING/NGENUITY_COMPONENTS/extracted/
REVERSE_ENGINEERING/NGENUITY_COMPONENTS/notes/
REVERSE_ENGINEERING/NGENUITY_COMPONENTS/dll_analysis/
REVERSE_ENGINEERING/NGENUITY_COMPONENTS/protocol_analysis/
```

## Ethical Guidelines

1. **Personal Use Only:** Extracted components are for personal study
2. **No Redistribution:** Do not share proprietary files
3. **No Public Commit:** Never commit to git repository
4. **Respect Copyright:** Acknowledge HP Inc. copyright
5. **Open Source Alternative:** Goal is to create open-source replacement

## Next Steps

1. Extract components using provided scripts
2. Analyze with Ghidra/dnSpy
3. Document findings in markdown files
4. Update HID_PROTOCOL.md with new discoveries
5. Implement improvements in HyperHeadsetv2 based on findings
6. Consider contacting Airoha for SDK access (if applicable)

## References

- NGENUITY Analysis: `../NGENUITY_ANALYSIS.md`
- Roadmap: `ROADMAP.md`
- HID Protocol: `HID_PROTOCOL.md`
- Linux Capture: `LINUX_HID_CAPTURE.md`
- macOS Capture: `MACOS_HID_CAPTURE.md`
