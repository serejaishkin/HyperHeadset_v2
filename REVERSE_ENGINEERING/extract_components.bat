@echo off
REM Extract NGENUITY components for reverse engineering
REM DO NOT commit extracted files to git repository

setlocal

set SRC="C:\Program Files\WindowsApps\33C30B79.HyperXNGenuity_5.38.0.0_x64__0a78dr3hq0pvt"
set DST="REVERSE_ENGINEERING\NGENUITY_COMPONENTS\extracted"

echo ====================================
echo NGENUITY Component Extraction
echo ====================================
echo.
echo Source: %SRC%
echo Destination: %DST%
echo.

REM Check if source exists
if not exist %SRC% (
    echo ERROR: NGENUITY not found at expected location
    echo Please install NGENUITY or update the path in this script
    pause
    exit /b 1
)

REM Create destination directory
if not exist "%DST%" mkdir "%DST%"

echo Extracting critical components...
echo.

REM Copy main DLL
echo [1/8] Copying NGenuity2.dll...
copy "%SRC%\NGenuity2.dll" "%DST%\" >nul 2>&1
if exist "%DST%\NGenuity2.dll" (
    echo      Success
) else (
    echo      Not found
)

REM Copy Airoha HID library (latest version)
echo [2/8] Copying AirohaHidCoreDll.dll (SDK 5.7.0.8)...
copy "%SRC%\Assets\Native\Airoha_SDK_5_7_0_8\AirohaHidCoreDll.dll" "%DST%\AirohaHidCoreDll.dll" >nul 2>&1
if exist "%DST%\AirohaHidCoreDll.dll" (
    echo      Success
) else (
    echo      Not found
)

REM Copy Airoha PEQ library
echo [3/8] Copying AirohaPeqLibrary.dll...
copy "%SRC%\Assets\Native\AirohaPeqLibrary.dll" "%DST%\" >nul 2>&1
if exist "%DST%\AirohaPeqLibrary.dll" (
    echo      Success
) else (
    echo      Not found
)

REM Copy DTS APO
echo [4/8] Copying DTSAPO.dll...
copy "%SRC%\Assets\Native\DTSAPO.dll" "%DST%\" >nul 2>&1
if exist "%DST%\DTSAPO.dll" (
    echo      Success
) else (
    echo      Not found
)

REM Copy helper executable
echo [5/8] Copying NGenuity2Helper.exe...
copy "%SRC%\NGenuity2Helper.exe" "%DST%\" >nul 2>&1
if exist "%DST%\NGenuity2Helper.exe" (
    echo      Success
) else (
    echo      Not found
)

REM Copy all SDK versions
echo [6/8] Copying all Airoha SDK versions...
if not exist "%DST%\sdk_versions" mkdir "%DST%\sdk_versions"
xcopy "%SRC%\Assets\Native\Airoha_SDK_*" "%DST%\sdk_versions\" /E /I /Y >nul 2>&1
if exist "%DST%\sdk_versions\Airoha_SDK_5_7_0_8" (
    echo      Success
) else (
    echo      Not found
)

REM Copy DTS configs
echo [7/8] Copying DTS configuration files...
if not exist "%DST%\dts_configs" mkdir "%DST%\dts_configs"
xcopy "%SRC%\Assets\Native\AudioDTS" "%DST%\dts_configs\" /E /I /Y >nul 2>&1
if exist "%DST%\dts_configs\HyperX" (
    echo      Success
) else (
    echo      Not found
)

REM Copy XML manifests
echo [8/8] Copying XML manifests...
copy "%SRC%\AppxManifest.xml" "%DST%\" >nul 2>&1
copy "%SRC%\plugins.xml" "%DST%\" >nul 2>&1
copy "%SRC%\NGenuity2.xr.xml" "%DST%\" >nul 2>&1
if exist "%DST%\AppxManifest.xml" (
    echo      Success
) else (
    echo      Not found
)

echo.
echo ====================================
echo Extraction Complete!
echo ====================================
echo.
echo Files saved to: %DST%
echo.
echo IMPORTANT:
echo   - DO NOT commit these files to git repository
echo   - These are proprietary files from HP Inc.
echo   - Use only for personal reverse engineering
echo   - Delete after analysis if desired
echo.
echo Next steps:
echo   1. Open DLLs in Ghidra or IDA Pro
echo   2. Document findings in REVERSE_ENGINEERING/NGENUITY_COMPONENTS/notes/
echo   3. Update HID_PROTOCOL.md with discoveries
echo.
pause
