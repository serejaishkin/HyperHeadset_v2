const { invoke } = window.__TAURI__.core;
const listen = window.__TAURI__.event ? window.__TAURI__.event.listen : window.__TAURI__.core.listen;

const $ = (id) => document.getElementById(id);
function tr(key) { const lang = window.__currentLang || 'ru'; return window.I18N?.[lang]?.[key] ?? window.I18N?.ru?.[key] ?? key; }

function updateCompact(state) {
    const connected = !!state?.connected;
    const pct = Math.max(0, Math.min(100, Number(state?.battery_percent ?? 0)));
    const connection = $('connection');
    const battery = $('battery-pct');
    const status = $('battery-st');
    const bar = $('progress-bar');
    const mic = $('mic-status');
    const sidetone = $('sidetone-c');

    connection.textContent = connected ? tr('conn.on') : tr('conn.off');
    connection.className = `connection ${connected ? 'on' : 'off'}`;
    battery.textContent = connected ? `${pct}%` : '--%';

    if (!connected) {
        status.textContent = tr('bat.no_connection');
        bar.style.width = '0%';
        bar.style.background = '#444';
        mic.textContent = '🎙️ ' + tr('bat.inactive');
        if (sidetone) sidetone.checked = false;
        return;
    }

    status.textContent = state.charging ? '⚡ ' + tr('bat.charging') : '🔋 ' + tr('bat.battery');
    bar.style.width = `${pct}%`;
    bar.style.background = state.charging ? '#20e83a' : pct > 30 ? '#35d07f' : pct > 15 ? '#ff9800' : '#f44336';
    mic.textContent = state.muted ? '🔇 ' + tr('mic.off') : '🎙️ ' + tr('mic.on');
    if (sidetone) sidetone.checked = !!state.sidetone;
}

async function refreshCompact() {
    try { updateCompact(await invoke('get_device_state')); }
    catch (error) { console.error('get_device_state failed', error); }
}

$('btn-mute-c').addEventListener('click', async () => {
    try { await invoke('toggle_mute'); }
    catch (error) { console.error('toggle_mute failed', error); }
});

$('btn-main-c').addEventListener('click', async () => {
    try { await invoke('show_main_window_cmd'); }
    catch (error) { console.error('show_main_window_cmd failed', error); window.close(); }
});

$('vol-master').addEventListener('input', async (e) => {
    $('vol-value').textContent = `${e.target.value}%`;
    try { await invoke('set_volume', { percent: Number(e.target.value) }); } catch (error) { console.debug(error); }
});

$('vol-mic').addEventListener('input', async (e) => {
    $('mic-value').textContent = `${e.target.value}%`;
    try { await invoke('set_mic_volume', { percent: Number(e.target.value) }); } catch (error) { console.debug(error); }
});

$('sidetone-c').addEventListener('change', async (e) => {
    try { await invoke('set_sidetone', { enabled: e.target.checked }); }
    catch (error) { console.error('set_sidetone failed', error); }
});

listen('device-state', e => updateCompact(e.payload));
listen('device-disconnected', () => updateCompact({ connected:false }));
listen('device-command-error', e => console.error('Device command failed:', e.payload));

async function loadAudioDevices() {
  try {
    const devices = await invoke('get_audio_devices');
    const outputSelect = $('fallback-output-c');
    const inputSelect = $('fallback-input-c');

    if (outputSelect) {
      const currentOutput = outputSelect.value;
      outputSelect.innerHTML = '<option value="">Нет</option>';
      devices.filter(d => d.is_output).forEach(d => {
        const opt = document.createElement('option');
        opt.value = d.id;
        opt.textContent = d.name;
        outputSelect.appendChild(opt);
      });
      outputSelect.value = currentOutput;
    }

    if (inputSelect) {
      const currentInput = inputSelect.value;
      inputSelect.innerHTML = '<option value="">Нет</option>';
      devices.filter(d => d.is_input).forEach(d => {
        const opt = document.createElement('option');
        opt.value = d.id;
        opt.textContent = d.name;
        inputSelect.appendChild(opt);
      });
      inputSelect.value = currentInput;
    }
  } catch (error) {
    console.error('loadAudioDevices failed', error);
  }
}

async function loadConfig() {
  try {
    const c = await invoke('get_config');
    const lang = c.language || 'ru';
    window.__currentLang = lang;
    document.documentElement.lang = lang;
    if (typeof window.applyLanguage === 'function') window.applyLanguage(lang);

    if ($('fallback-output-c')) $('fallback-output-c').value = c.audio?.fallback_output_device ?? '';
    if ($('fallback-input-c')) $('fallback-input-c').value = c.audio?.fallback_input_device ?? '';
    if ($('auto-switch-c')) $('auto-switch-c').checked = !!c.audio?.auto_switch_audio;
  } catch (e) { console.error('loadConfig failed', e); }
}

async function saveConfig() {
  try {
    const c = await invoke('get_config');
    c.audio.fallback_output_device = $('fallback-output-c').value || null;
    c.audio.fallback_input_device = $('fallback-input-c').value || null;
    c.audio.auto_switch_audio = $('auto-switch-c').checked;
    await invoke('save_config', { config: c });
  } catch (e) { console.error('saveConfig failed', e); }
}

$('fallback-output-c').addEventListener('change', saveConfig);
$('fallback-input-c').addEventListener('change', saveConfig);
$('auto-switch-c').addEventListener('change', saveConfig);

refreshCompact();
loadConfig();
loadAudioDevices();
setInterval(refreshCompact, 2000);
