import os
import shutil

voice_dir = r"D:\GitHub\HyperHeadsetv2\assets\voice"
src_template = os.path.join(voice_dir, "bat_000.wav")

if os.path.exists(src_template):
    for i in range(1, 100):
        filename = f"bat_{i:03d}.wav"
        dest_path = os.path.join(voice_dir, filename)
        if not os.path.exists(dest_path):
            shutil.copy(src_template, dest_path)
    print("Generated 1-99 wav files successfully.")

rs_path = r"D:\GitHub\HyperHeadsetv2\src\audio\embedded_voice.rs"
lines = []
lines.append("// Auto-generated embedded voice module\n")

for i in range(101):
    name = f"BAT_{i:03d}"
    fname = f"bat_{i:03d}.wav"
    lines.append(f'pub const {name}: &[u8] = include_bytes!("../../assets/voice/{fname}");\n')

lines.append('pub const CHARGING: &[u8] = include_bytes!("../../assets/voice/charging.wav");\n')
lines.append('pub const FULL_CHARGE: &[u8] = include_bytes!("../../assets/voice/full_charge.wav");\n')
lines.append('pub const LOW_BATTERY: &[u8] = include_bytes!("../../assets/voice/low_battery.wav");\n')

lines.append('\npub fn get(percent: u8) -> &\'static [u8] {\n')
lines.append('    match percent {\n')
for i in range(101):
    name = f"BAT_{i:03d}"
    lines.append(f'        {i} => {name},\n')
lines.append('        _ => BAT_100,\n')
lines.append('    }\n')
lines.append('}\n')

with open(rs_path, "w", encoding="utf-8") as f:
    f.writelines(lines)

print("Generated src/audio/embedded_voice.rs successfully.")
