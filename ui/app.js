const { invoke, listen } = window.__TAURI__.core;

const $ = (id) => document.getElementById(id);
let toastTimer = null;
let config = null;
let trayConfig = null;
let dirty = false;

function toast(message) {
  let node = $('toast');
  if (!node) {
    node = document.createElement('div'); node.id = 'toast'; node.className = 'toast'; document.body.appendChild(node);
  }
  node.textContent = message; node.classList.add('show'); clearTimeout(toastTimer);
  toastTimer = setTimeout(() => node.classList.remove('show'), 3000);
}
function markDirty() { dirty = true; $('settings-dirty').textContent = 'Unsaved changes'; $('settings-dirty').classList.add('dirty'); $('save-message').textContent = 'There are unsaved changes'; }
function markSaved() { dirty = false; $('settings-dirty').textContent = 'Saved'; $('settings-dirty').classList.remove('dirty'); $('save-message').textContent = 'Changes are saved to config.toml and tray_icon.toml'; }

function setConnection(mode, battery = null) {
  const state = $('connection-state'); state.className = `connection-state ${mode}`;
  state.textContent = mode === 'connected' ? 'ON' : mode === 'searching' ? 'SEARCHING' : 'OFF';
  $('header-battery').textContent = battery == null ? '[BAT] --%' : `[BAT] ${battery}%`;
}

function updateBattery(device) {
  const connected = !!device?.connected, percent = Number(device?.battery_percent ?? 0), charging = !!device?.charging, fill = $('battery-bar-fill');
  if (!connected) {
    $('battery-percent').textContent = '--%'; $('battery-status').textContent = 'No connection';
    $('mic-status').textContent = 'Inactive'; $('mic-status').className = 'status-value inactive'; $('signal-value').textContent = '-- dBm';
    fill.style.width = '0%'; fill.className = ''; setConnection('disconnected');
    $('device-message').className = 'device-message'; $('device-message-title').textContent = 'Headset disconnected'; $('device-message-text').textContent = 'Waiting for the device...'; return;
  }
  const pct = Math.max(0, Math.min(100, percent)); $('battery-percent').textContent = `${pct}%`;
  $('battery-status').textContent = charging ? 'Charging' : 'Battery'; fill.style.width = `${pct}%`; fill.className = charging ? 'charging' : pct <= 20 ? 'low' : '';
  $('mic-status').textContent = device.muted ? 'Muted' : 'Active'; $('mic-status').className = device.muted ? 'status-value inactive' : 'status-value';
  $('signal-value').textContent = `${Number(device.signal_dbm ?? 0)} dBm`; $('sidetone').checked = !!device.sidetone;
  $('mic-toggle').textContent = device.muted ? 'MIC OFF' : 'MIC ON'; $('mic-toggle').classList.toggle('active', !!device.muted);
  setConnection('connected', pct); $('device-message').className = 'device-message connected'; $('device-message-title').textContent = 'Headset connected'; $('device-message-text').textContent = charging ? 'Charging' : 'Device is ready';
}

async function refreshDevices() {
  try {
    const devices = await invoke('get_connected_devices');
    const container = $('device-selector-container');
    const select = $('device-select');
    if (devices && devices.length > 1) {
      container.style.display = 'block';
      const currentVal = select.value;
      select.innerHTML = '';
      devices.forEach((d, idx) => {
        const opt = document.createElement('option');
        opt.value = idx;
        opt.textContent = `${d.name || 'Headset #' + (idx+1)} (${d.battery_percent}%)`;
        select.appendChild(opt);
      });
      if (currentVal !== '') select.value = currentVal;
    } else if (container) {
      container.style.display = 'none';
    }
  } catch (e) {
    // Ignore if not implemented or unavailable
  }
}

async function refresh() { try { updateBattery(await invoke('get_device_state')); refreshDevices(); } catch (error) { console.error('get_device_state failed', error); } }
async function command(name, args = {}) { try { return await invoke(name, args); } catch (error) { console.error(`${name} failed`, error); toast(`${name}: ${error}`); throw error; } }

async function refreshAudio() {
  try {
    const levels = await invoke('get_audio_levels');
    $('volume').value = levels.output; $('volume-value').textContent = `${levels.output}%`;
    $('mic-volume').value = levels.input; $('mic-value').textContent = `${levels.input}%`;
  } catch (error) { console.debug('System audio levels unavailable:', error); }
}

async function checkBatteryVoice() {
  try { await invoke('check_battery_voice'); toast('Battery voice notification played'); }
  catch (error) { console.error('check_battery_voice failed', error); toast(`Voice check failed: ${error}`); }
}

function rgbaToHex(a) { return `#${[0,1,2].map(i => Number(a?.[i] ?? 0).toString(16).padStart(2,'0')).join('')}`; }
function hexToRgba(hex, alpha = 255) { const h = String(hex || '#000000').replace('#',''); return [parseInt(h.slice(0,2),16)||0, parseInt(h.slice(2,4),16)||0, parseInt(h.slice(4,6),16)||0, alpha]; }
function setColor(id, value) { const el = $(id); if (el) el.value = rgbaToHex(value); }
function colorValue(id, fallback) { return hexToRgba($(id)?.value || '#000000', fallback?.[3] ?? 255); }

function trayToUi(t) {
  trayConfig = structuredClone(t);
  $('tray-size').value = t.size; $('tray-font-scale').value = t.font_scale; $('tray-outline').value = t.outline_width; $('tray-border').value = t.border_width; $('tray-gap').value = t.gap_between_digits;
  for (const name of ['charging','high','medium','low']) {
    setColor(`tray-${name}-bg`, t.colors[name].bg); setColor(`tray-${name}-fg`, t.colors[name].fg); setColor(`tray-${name}-outline`, t.colors[name].outline); setColor(`tray-${name}-border`, t.colors[name].border);
  }
}
function uiToTray() {
  const t = structuredClone(trayConfig || {});
  t.size = Number($('tray-size').value) || 256; t.font_scale = Number($('tray-font-scale').value) || 8; t.outline_width = Number($('tray-outline').value) || 0; t.border_width = Number($('tray-border').value) || 0; t.gap_between_digits = Number($('tray-gap').value) || 0;
  if (!t.colors) return t;
  for (const name of ['charging','high','medium','low']) {
    t.colors[name].bg = colorValue(`tray-${name}-bg`, t.colors[name].bg); t.colors[name].fg = colorValue(`tray-${name}-fg`, t.colors[name].fg); t.colors[name].outline = colorValue(`tray-${name}-outline`, t.colors[name].outline); t.colors[name].border = colorValue(`tray-${name}-border`, t.colors[name].border);
  }
  return t;
}

function configToUi(c) {
  config = structuredClone(c);
  $('cfg-enabled').checked = !!c.enabled; $('cfg-sidetone').checked = !!c.device?.sidetone; $('cfg-voice-prompts').checked = !!c.device?.voice_prompts;
  $('cfg-auto-shutdown').value = c.device?.auto_shutdown_minutes ?? 30; $('cfg-mute-mode').value = c.input?.mute_button_mode ?? 'SmartDouble'; $('cfg-keybind').value = c.keybind ?? 'F20'; $('cfg-double-tap').value = c.double_tap_ms ?? 500;
  $('voice-enabled').checked = !!c.voice?.enabled; $('voice-battery-low').checked = !!c.voice?.on_battery_low; $('voice-charging').checked = !!c.voice?.on_charging; $('voice-full-charge').checked = !!c.voice?.on_full_charge;
  $('voice-connected').checked = !!c.voice?.on_connected; $('voice-disconnected').checked = !!c.voice?.on_disconnected; $('voice-button-check').checked = c.voice?.on_button_check !== false; $('voice-exact-percent').checked = !!c.voice?.exact_percent;
  $('discord-mode').value = c.discord?.mode ?? 'Keybind'; $('discord-keybind').value = c.discord?.keybind ?? 'F20'; $('discord-app-id').value = c.discord?.direct?.app_id ?? ''; $('discord-battery').checked = !!c.discord?.direct?.show_battery; $('discord-mute').checked = !!c.discord?.direct?.show_mute_status;
  $('cfg-debug').checked = !!c.debug_logging; $('cfg-console').checked = !!c.log_to_console; $('cfg-file').checked = !!c.log_to_file; $('cfg-start-os').checked = !!c.start_with_os; $('cfg-start-compact').checked = !!c.start_in_compact_mode; $('cfg-compact').checked = !!c.compact_mode; $('cfg-language').value = c.language ?? 'ru';
  const bands = c.audio?.eq_bands ?? Array(10).fill(0); document.querySelectorAll('.eq-band input').forEach((input, i) => input.value = Number(bands[i] ?? 0));
  $('sidetone').checked = !!c.device?.sidetone; markSaved();
}

function uiToConfig() {
  const c = structuredClone(config); c.enabled = $('cfg-enabled').checked; c.keybind = $('cfg-keybind').value.trim() || 'F20'; c.double_tap_ms = Number($('cfg-double-tap').value) || 500;
  c.compact_mode = $('cfg-compact').checked; c.start_with_os = $('cfg-start-os').checked; c.start_in_compact_mode = $('cfg-start-compact').checked; c.language = $('cfg-language').value;
  c.device.sidetone = $('cfg-sidetone').checked; c.device.voice_prompts = $('cfg-voice-prompts').checked; c.device.auto_shutdown_minutes = Number($('cfg-auto-shutdown').value) || 0; c.input.mute_button_mode = $('cfg-mute-mode').value;
  c.voice.enabled = $('voice-enabled').checked; c.voice.on_battery_low = $('voice-battery-low').checked; c.voice.on_charging = $('voice-charging').checked; c.voice.on_full_charge = $('voice-full-charge').checked; c.voice.on_connected = $('voice-connected').checked; c.voice.on_disconnected = $('voice-disconnected').checked; c.voice.on_button_check = $('voice-button-check').checked; c.voice.exact_percent = $('voice-exact-percent').checked;
  c.discord.mode = $('discord-mode').value; c.discord.keybind = $('discord-keybind').value.trim() || null; c.discord.direct.app_id = $('discord-app-id').value.trim(); c.discord.direct.show_battery = $('discord-battery').checked; c.discord.direct.show_mute_status = $('discord-mute').checked;
  c.debug_logging = $('cfg-debug').checked; c.log_to_console = $('cfg-console').checked; c.log_to_file = $('cfg-file').checked;
  c.audio.eq_bands = Array.from(document.querySelectorAll('.eq-band input')).map(input => Number(input.value)); return c;
}

async function loadConfig() {
  try {
    const [c, t] = await Promise.all([invoke('get_config'), invoke('get_tray_config')]);
    configToUi(c); trayToUi(t);
    if (c.start_in_compact_mode) setTimeout(() => invoke('open_compact_window').catch(e => console.debug('compact startup:', e)), 250);
  }
  catch (error) { console.error('Settings load failed', error); toast(`Settings load failed: ${error}`); }
}

async function saveSettings() {
  const newConfig = uiToConfig(), newTray = uiToTray();
  try {
    await invoke('save_config', { config: newConfig }); config = newConfig;
    await invoke('save_tray_config', { config: newTray }); trayConfig = newTray;
    try { await invoke('apply_eq', { bands: newConfig.audio.eq_bands }); }
    catch (eqError) { console.info('EQ apply:', eqError); }
    await command('set_sidetone', { enabled: newConfig.device.sidetone }).catch(() => {});
    markSaved(); toast('Settings saved');
  } catch (error) { console.error('Save failed', error); toast(`Save failed: ${error}`); }
}

$('btn-mute').addEventListener('click', () => command('toggle_mute'));
$('btn-check').addEventListener('click', async () => { await refresh(); await refreshAudio(); });
$('btn-reconnect').addEventListener('click', refresh);
$('btn-voice-check').addEventListener('click', checkBatteryVoice);
$('btn-test-voice').addEventListener('click', async () => { try { await invoke('test_voice'); toast('Bundled WAV test started'); } catch (e) { toast(`Voice test failed: ${e}`); } });
$('btn-save-settings').addEventListener('click', saveSettings); $('btn-reset-settings').addEventListener('click', loadConfig); $('btn-compact').addEventListener('click', () => command('open_compact_window'));
$('sidetone').addEventListener('change', (e) => { command('set_sidetone', { enabled: e.target.checked }); if (config) { config.device.sidetone = e.target.checked; $('cfg-sidetone').checked = e.target.checked; markDirty(); } });
$('volume').addEventListener('input', async e => { $('volume-value').textContent = `${e.target.value}%`; try { await invoke('set_volume', { percent: Number(e.target.value) }); } catch (err) { console.debug(err); } });
$('mic-volume').addEventListener('input', async e => { $('mic-value').textContent = `${e.target.value}%`; try { await invoke('set_mic_volume', { percent: Number(e.target.value) }); } catch (err) { console.debug(err); } });
$('mic-toggle').addEventListener('click', () => command('toggle_mute'));
$('play-button').addEventListener('click', () => command('play_pause'));
const devSelect = $('device-select');
if (devSelect) {
  devSelect.addEventListener('change', async (e) => {
    try {
      await invoke('select_device', { index: Number(e.target.value) });
      refresh();
    } catch (err) {
      toast(`Failed to switch device: ${err}`);
    }
  });
}

document.querySelectorAll('#settings input, #settings select').forEach(el => { el.addEventListener('change', markDirty); el.addEventListener('input', markDirty); });
for (const button of document.querySelectorAll('.tab-btn')) button.addEventListener('click', () => { document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active')); document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active')); button.classList.add('active'); $(button.dataset.tab).classList.add('active'); });
for (const button of document.querySelectorAll('.settings-tab')) button.addEventListener('click', () => { document.querySelectorAll('.settings-tab').forEach(b => b.classList.remove('active')); document.querySelectorAll('.settings-pane').forEach(p => p.classList.remove('active')); button.classList.add('active'); $(button.dataset.settingsTab).classList.add('active'); });

const presets = { Flat:[0,0,0,0,0,0,0,0,0,0], 'Bass Boost':[6,4,2,0,0,0,0,0,0,0], 'Bass Cut':[-6,-4,-2,0,0,0,0,0,0,0], 'Treble Boost':[0,0,0,0,0,0,2,4,6,8], 'Voice Chat':[-2,0,2,4,6,6,4,2,0,-2], Gaming:[4,3,2,1,0,0,1,2,3,4] };
for (const band of document.querySelectorAll('.eq-band input')) band.addEventListener('input', markDirty);
$('eq-reset').addEventListener('click', () => { document.querySelectorAll('.eq-band input').forEach(i => i.value = 0); markDirty(); });
$('eq-preset').addEventListener('change', () => { const values = presets[$('eq-preset').value] || presets.Flat; document.querySelectorAll('.eq-band input').forEach((input, i) => input.value = values[i]); markDirty(); });
$('eq-apply').addEventListener('click', async () => { const bands = Array.from(document.querySelectorAll('.eq-band input')).map(i => Number(i.value)); try { await invoke('apply_eq', { bands }); toast('EQ applied'); } catch (e) { toast(`EQ: ${e}`); } });

let activeDeviceId = null;
async function refreshPerDeviceEq() {
  try {
    const st = await invoke('get_device_state');
    activeDeviceId = st.connected ? (st.device_id || '') : null;
    const note = $('eq-device-note'), btnClear = $('eq-clear-per-device');
    if (!activeDeviceId) { note.style.display = 'none'; btnClear.style.display = 'none'; return; }
    const per = await invoke('get_per_device_config', { deviceId: activeDeviceId });
    note.style.display = 'block';
    note.textContent = per && per.audio
      ? `Per-device EQ preset is active for "${per.name || 'this headset'}"`
      : `Using global EQ for "${st.name || 'this headset'}"`;
    btnClear.style.display = per && per.audio ? 'inline-block' : 'none';
  } catch (e) { console.debug('per-device eq:', e); }
}
$('eq-save-per-device').addEventListener('click', async () => {
  if (!activeDeviceId) { toast('No connected device'); return; }
  try {
    const bands = Array.from(document.querySelectorAll('.eq-band input')).map(i => Number(i.value));
    const st = await invoke('get_device_state');
    let per = null;
    try { per = await invoke('get_per_device_config', { deviceId: activeDeviceId }); } catch (_) {}
    per = Object.assign({ name: '', audio: null, device: null, voice: null }, per || {});
    per.name = st.name || per.name;
    per.audio = { eq_bands: bands };
    await invoke('save_per_device_config', { deviceId: activeDeviceId, perCfg: per });
    toast('Per-device EQ saved'); refreshPerDeviceEq();
  } catch (e) { toast(`Save failed: ${e}`); }
});
$('eq-clear-per-device').addEventListener('click', async () => {
  if (!activeDeviceId) return;
  try {
    let per = await invoke('get_per_device_config', { deviceId: activeDeviceId });
    if (per) { per.audio = null; await invoke('save_per_device_config', { deviceId: activeDeviceId, perCfg: per }); }
    toast('Per-device EQ removed'); refreshPerDeviceEq();
  } catch (e) { toast(`Remove failed: ${e}`); }
});
refreshPerDeviceEq();

async function loadVoiceDir() {
  try {
    const cfg = await invoke('get_config');
    $('voice-custom-dir').value = cfg.custom_voice_dir || '';
  } catch (e) { console.debug('voice dir load:', e); }
}
const btnSaveVoiceDir = $('btn-save-voice-dir');
if (btnSaveVoiceDir) btnSaveVoiceDir.addEventListener('click', async () => {
  try { await invoke('set_custom_voice_dir', { path: $('voice-custom-dir').value.trim() }); toast('Custom voice folder saved'); }
  catch (e) { toast(`Save failed: ${e}`); }
});
const voiceUpload = $('voice-upload-file');
if (voiceUpload) voiceUpload.addEventListener('change', async (e) => {
  const file = e.target.files[0];
  if (!file) return;
  try {
    const buf = new Uint8Array(await file.arrayBuffer());
    const saved = await invoke('upload_voice_file', { filename: file.name, data: Array.from(buf) });
    toast(`Uploaded: ${saved}`);
  } catch (err) { toast(`Upload failed: ${err}`); }
  e.target.value = '';
});
loadVoiceDir();

const btnUpdates = $('btn-check-updates');
if (btnUpdates) btnUpdates.addEventListener('click', async () => {
  try {
    const updater = window.__TAURI__.updater;
    const relaunch = window.__TAURI__.process ? window.__TAURI__.process.relaunch : null;
    if (!updater || !updater.check) { toast('Updater not available'); return; }
    toast('Checking for updates...');
    const update = await updater.check();
    if (!update) { toast('You are up to date'); return; }
    toast(`Downloading update ${update.version}...`);
    await update.downloadAndInstall();
    if (relaunch) await relaunch();
  } catch (e) {
    console.error('update check failed:', e);
    toast(`Update check failed: ${e}`);
  }
});

listen('device-state', event => updateBattery(event.payload));
listen('device-connected', refresh);
listen('device-disconnected', () => updateBattery({ connected:false }));
listen('device-command-error', event => toast(`Device command failed: ${event.payload}`));
listen('device-command-ok', event => { if (event.payload === 'mute') toast('Mute state changed'); });

loadConfig();
refresh();
refreshAudio();
setInterval(refresh, 1000);
setInterval(refreshAudio, 2000);
