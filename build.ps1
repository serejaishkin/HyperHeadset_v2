# build-windows.ps1
# Run from repo root after `cargo build --release`
param(
    [switch]$InstallInnoSetup
)

$ErrorActionPreference = "Stop"

Write-Host "=== Building HyperX NGENUITY Open Installer ===" -ForegroundColor Cyan

# 1. Build release
Write-Host "[1/3] Building Rust release..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# 2. Check Inno Setup
$ISCC = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
if (-not (Test-Path $ISCC)) {
    $ISCC = "${env:ProgramFiles}\Inno Setup 6\ISCC.exe"
}

if (-not (Test-Path $ISCC)) {
    Write-Host "Inno Setup 6 not found." -ForegroundColor Red
    if ($InstallInnoSetup) {
        Write-Host "Downloading Inno Setup..." -ForegroundColor Yellow
        $url = "https://jrsoftware.org/download.php/is.exe"
        $out = "$env:TEMP\is.exe"
        Invoke-WebRequest -Uri $url -OutFile $out
        Start-Process -FilePath $out -ArgumentList "/VERYSILENT","/NORESTART" -Wait
        $ISCC = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
    } else {
        Write-Host "Install from https://jrsoftware.org/isdl.php or run with -InstallInnoSetup"
        exit 1
    }
}

# 3. Compile installer
Write-Host "[2/3] Compiling installer..." -ForegroundColor Yellow
& $ISCC "installer\windows\HyperXSetup.iss"
if ($LASTEXITCODE -ne 0) { throw "ISCC failed" }

# 4. Result
$Installer = Get-ChildItem "installer\windows\output\*.exe" | Select-Object -First 1
Write-Host "[3/3] Done! Installer: $($Installer.FullName)" -ForegroundColor Green
