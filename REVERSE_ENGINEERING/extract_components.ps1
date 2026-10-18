# PowerShell script to extract NGENUITY components
# DO NOT commit extracted files to git repository

$ErrorActionPreference = "Stop"

$Source = "C:\Program Files\WindowsApps\33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt"
$Destination = "REVERSE_ENGINEERING\NGENUITY_COMPONENTS\extracted"

Write-Host "====================================" -ForegroundColor Cyan
Write-Host "NGENUITY Component Extraction" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Source: $Source" -ForegroundColor Gray
Write-Host "Destination: $Destination" -ForegroundColor Gray
Write-Host ""

# Check if source exists
if (-not (Test-Path $Source)) {
    Write-Host "ERROR: NGENUITY not found at expected location" -ForegroundColor Red
    Write-Host "Please install NGENUITY or update the path in this script" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}

# Create destination directory
New-Item -ItemType Directory -Path $Destination -Force | Out-Null

Write-Host "Extracting critical components..." -ForegroundColor Cyan
Write-Host ""

# Define files to copy
$FilesToCopy = @(
    @{ Src = "NGenuity2.dll"; Dest = "NGenuity2.dll" }
    @{ Src = "Assets\Native\Airoha_SDK_5_7_0_8\AirohaHidCoreDll.dll"; Dest = "AirohaHidCoreDll.dll" }
    @{ Src = "Assets\Native\AirohaPeqLibrary.dll"; Dest = "AirohaPeqLibrary.dll" }
    @{ Src = "Assets\Native\DTSAPO.dll"; Dest = "DTSAPO.dll" }
    @{ Src = "NGenuity2Helper.exe"; Dest = "NGenuity2Helper.exe" }
)

$Progress = 1
$Total = $FilesToCopy.Count + 3  # +3 for SDK, DTS, and XML

foreach ($File in $FilesToCopy) {
    $SrcPath = Join-Path $Source $File.Src
    $DstPath = Join-Path $Destination $File.Dest
    
    Write-Host "[$Progress/$Total] Copying $($File.Dest)..." -NoNewline
    
    if (Test-Path $SrcPath) {
        Copy-Item $SrcPath $DstPath -Force
        Write-Host " Success" -ForegroundColor Green
    } else {
        Write-Host " Not found" -ForegroundColor Yellow
    }
    
    $Progress++
}

# Copy all SDK versions
Write-Host "[$Progress/$Total] Copying all Airoha SDK versions..." -NoNewline
$SDKDir = Join-Path $Destination "sdk_versions"
New-Item -ItemType Directory -Path $SDKDir -Force | Out-Null

$SDKCopied = $false
Get-ChildItem "$Source\Assets\Native\Airoha_SDK_*" -ErrorAction SilentlyContinue | ForEach-Object {
    Copy-Item $_.FullName $SDKDir -Recurse -Force
    $SDKCopied = $true
}

if ($SDKCopied) {
    Write-Host " Success" -ForegroundColor Green
} else {
    Write-Host " Not found" -ForegroundColor Yellow
}

$Progress++

# Copy DTS configs
Write-Host "[$Progress/$Total] Copying DTS configuration files..." -NoNewline
$DTSDir = Join-Path $Destination "dts_configs"
New-Item -ItemType Directory -Path $DTSDir -Force | Out-Null

$DTSCopied = $false
Get-ChildItem "$Source\Assets\Native\AudioDTS" -ErrorAction SilentlyContinue | ForEach-Object {
    Copy-Item $_.FullName $DTSDir -Recurse -Force
    $DTSCopied = $true
}

if ($DTSCopied) {
    Write-Host " Success" -ForegroundColor Green
} else {
    Write-Host " Not found" -ForegroundColor Yellow
}

$Progress++

# Copy XML manifests
Write-Host "[$Progress/$Total] Copying XML manifests..." -NoNewline
$XMLCopied = $false

$XMLFiles = @("AppxManifest.xml", "plugins.xml", "NGenuity2.xr.xml")
foreach ($XMLFile in $XMLFiles) {
    $SrcPath = Join-Path $Source $XMLFile
    if (Test-Path $SrcPath) {
        Copy-Item $SrcPath $Destination -Force
        $XMLCopied = $true
    }
}

if ($XMLCopied) {
    Write-Host " Success" -ForegroundColor Green
} else {
    Write-Host " Not found" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "====================================" -ForegroundColor Cyan
Write-Host "Extraction Complete!" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Files saved to: $Destination" -ForegroundColor Gray
Write-Host ""
Write-Host "IMPORTANT:" -ForegroundColor Red
Write-Host "  - DO NOT commit these files to git repository" -ForegroundColor Red
Write-Host "  - These are proprietary files from HP Inc." -ForegroundColor Red
Write-Host "  - Use only for personal reverse engineering" -ForegroundColor Red
Write-Host "  - Delete after analysis if desired" -ForegroundColor Red
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Open DLLs in Ghidra or IDA Pro" -ForegroundColor Gray
Write-Host "  2. Document findings in REVERSE_ENGINEERING/NGENUITY_COMPONENTS/notes/" -ForegroundColor Gray
Write-Host "  3. Update HID_PROTOCOL.md with discoveries" -ForegroundColor Gray
Write-Host ""
Read-Host "Press Enter to exit"
