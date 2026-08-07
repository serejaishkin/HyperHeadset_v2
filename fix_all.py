with open('src/gui/mod.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
for i, line in enumerate(lines):
    new_lines.append(line)
    
    # After "VOL" slider block, add MIC slider
    if 'ui.label(self.i18n.t("VOL"));' in line:
        # Skip the next 6 lines (slider + if block) then add MIC
        pass  # We'll handle this differently
    
    # Fix expand button in compact mode
    if 'self.config.compact_mode = false;' in line and i > 400:
        new_lines.append('                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([980.0, 600.0].into()));\n')
        new_lines.append('                    ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));\n')
    
    # Fix compact button in top panel
    if 'self.needs_save = true;' in line and i > 350 and i < 360:
        new_lines.append('                        if !self.config.compact_mode {\n')
        new_lines.append('                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([980.0, 600.0].into()));\n')
        new_lines.append('                        }\n')

# Now handle MIC volume insertion - find the right spot
result = []
skip = 0
for i, line in enumerate(new_lines):
    if skip > 0:
        skip -= 1
        result.append(line)
        continue
    
    result.append(line)
    
    # After VOL block ends (after controller.set_master_volume), add MIC
    if 'controller.set_master_volume(vol);' in line:
        # Skip next 3 lines (closing braces)
        skip = 3
        result.append(new_lines[i+1] if i+1 < len(new_lines) else '')  # }
        result.append(new_lines[i+2] if i+2 < len(new_lines) else '')  # }
        result.append(new_lines[i+3] if i+3 < len(new_lines) else '')  # ui.add_space
        
        # Add MIC volume
        result.append('                ui.add_space(5.0);\n')
        result.append('                ui.label(self.i18n.t("MIC"));\n')
        result.append('                let mut mic_vol = self.mic_volume;\n')
        result.append('                ui.add(egui::Slider::new(&mut mic_vol, 0.0..=100.0).show_value(true).text(""));\n')
        result.append('                if mic_vol != self.mic_volume {\n')
        result.append('                    self.mic_volume = mic_vol;\n')
        result.append('                    if let Some(ref controller) = self.volume_controller {\n')
        result.append('                        controller.set_microphone_volume(mic_vol);\n')
        result.append('                    }\n')
        result.append('                }\n')

with open('src/gui/mod.rs', 'w') as f:
    f.writelines(result)

# Fix main.rs
with open('src/main.rs', 'r') as f:
    main = f.read()

main = main.replace(
    '// #![cfg_attr(target_os = "windows", windows_subsystem = "windows")] temporarily removed for debug',
    '#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]'
)

with open('src/main.rs', 'w') as f:
    f.write(main)

print("Done! Run: cargo build --release")