```powershell
Add-Type -AssemblyName System.Speech

# Папка рядом со скриптом
$outputDir = Join-Path $PSScriptRoot "battery_voices"

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer

# ------------------------------------------------------------
# Поиск женского английского голоса
# ------------------------------------------------------------

$voices = $synth.GetInstalledVoices()

Write-Host ""
Write-Host "Installed voices:" -ForegroundColor Cyan

foreach ($v in $voices) {
    Write-Host "  $($v.VoiceInfo.Name) | $($v.VoiceInfo.Culture.Name) | $($v.VoiceInfo.Gender)"
}

# Сначала ищем английский Female
$voice = $voices |
    Where-Object {
        $_.VoiceInfo.Culture.Name -like "en-*" -and
        $_.VoiceInfo.Gender.ToString() -eq "Female"
    } |
    Select-Object -First 1

# Если Gender не определился — ищем популярные женские голоса по имени
if ($null -eq $voice) {
    $voice = $voices |
        Where-Object {
            $_.VoiceInfo.Culture.Name -like "en-*" -and
            $_.VoiceInfo.Name -match "Zira|Hazel|Samantha|Jenny|Aria|Female"
        } |
        Select-Object -First 1
}

# Если женского нет — берём любой английский
if ($null -eq $voice) {
    $voice = $voices |
        Where-Object {
            $_.VoiceInfo.Culture.Name -like "en-*"
        } |
        Select-Object -First 1
}

if ($null -eq $voice) {
    Write-Host ""
    Write-Host "ERROR: No English voice found." -ForegroundColor Red
    Write-Host ""
    Write-Host "Install an English speech voice in:"
    Write-Host "Settings -> Time & language -> Speech"
    exit 1
}

$synth.SelectVoice($voice.VoiceInfo.Name)

Write-Host ""
Write-Host "Selected voice:" -ForegroundColor Green
Write-Host $voice.VoiceInfo.Name -ForegroundColor Green
Write-Host ""

# ------------------------------------------------------------
# Настройки голоса
# ------------------------------------------------------------

$synth.Rate = 0
$synth.Volume = 100

# ------------------------------------------------------------
# 0-99%
# ------------------------------------------------------------

for ($i = 0; $i -le 99; $i++) {

    $filename = "bat_{0:D3}.wav" -f $i
    $path = Join-Path $outputDir $filename

    $text = "Battery $i percent."

    $synth.SetOutputToWaveFile($path)
    $synth.Speak($text)
    $synth.SetOutputToNull()

    Write-Host "$filename -> $text"
}

# ------------------------------------------------------------
# Статусы
# ------------------------------------------------------------

$statuses = @{
    "charging.wav"    = "Battery charging."
    "full_charge.wav" = "Battery fully charged."
    "low_battery.wav" = "Low battery."
}

foreach ($item in $statuses.GetEnumerator()) {

    $filename = $item.Key
    $text = $item.Value
    $path = Join-Path $outputDir $filename

    $synth.SetOutputToWaveFile($path)
    $synth.Speak($text)
    $synth.SetOutputToNull()

    Write-Host "$filename -> $text"
}

$synth.Dispose()

Write-Host ""
Write-Host "====================================" -ForegroundColor Green
Write-Host "DONE!" -ForegroundColor Green
Write-Host "103 WAV files generated." -ForegroundColor Green
Write-Host "====================================" -ForegroundColor Green
Write-Host ""
Write-Host "Folder:"
Write-Host $outputDir
```
