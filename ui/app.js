const { invoke } = window.__TAURI__.core;
const listen = window.__TAURI__.event ? window.__TAURI__.event.listen : window.__TAURI__.core.listen;

const $ = (id) => document.getElementById(id);
let toastTimer = null;
let config = null;
let trayConfig = null;
let dirty = false;
let currentBattery = 0;
let currentCharging = false;

function tr(key) { const lang = window.__currentLang || 'ru'; return window.I18N?.[lang]?.[key] ?? window.I18N?.ru?.[key] ?? key; }

function toast(message) {
  let node = $('toast');
  if (!node) {
    node = document.createElement('div'); node.id = 'toast'; node.className = 'toast'; document.body.appendChild(node);
  }
  node.textContent = message; node.classList.add('show'); clearTimeout(toastTimer);
  toastTimer = setTimeout(() => node.classList.remove('show'), 3000);
}
function markDirty() { dirty = true; $('settings-dirty').textContent = tr('settings.unsaved'); $('settings-dirty').classList.add('dirty'); $('save-message').textContent = tr('settings.unsaved_msg'); }
function markSaved() { dirty = false; $('settings-dirty').textContent = tr('settings.saved'); $('settings-dirty').classList.remove('dirty'); $('save-message').textContent = tr('settings.save_msg'); }

function setConnection(mode, battery = null) {
  const state = $('connection-state'); state.className = `connection-state ${mode}`;
  state.textContent = mode === 'connected' ? tr('conn.on') : mode === 'searching' ? tr('conn.searching') : tr('conn.off');
  $('header-battery').textContent = battery == null ? `${tr('header.battery')} --%` : `${tr('header.battery')} ${battery}%`;
}

function updateBattery(device) {
  const connected = !!device?.connected, percent = Number(device?.battery_percent ?? 0), charging = !!device?.charging, fill = $('battery-bar-fill');
  currentBattery = percent;
  currentCharging = charging;
  if (!connected) {
    $('battery-percent').textContent = '--%'; $('battery-status').textContent = tr('bat.no_connection');
    $('mic-status').textContent = tr('bat.inactive'); $('mic-status').className = 'status-value inactive'; $('signal-value').textContent = '-- dBm';
    fill.style.width = '0%'; fill.className = ''; setConnection('disconnected');
    $('device-message').className = 'device-message'; $('device-message-title').textContent = tr('dash.disconnected_title'); $('device-message-text').textContent = tr('dash.disconnected_text'); return;
  }
  const pct = Math.max(0, Math.min(100, percent)); $('battery-percent').textContent = `${pct}%`;
  $('battery-status').textContent = charging ? tr('bat.charging') : tr('bat.battery'); fill.style.width = `${pct}%`; fill.className = charging ? 'charging' : pct <= 20 ? 'low' : '';
  $('mic-status').textContent = device.muted ? tr('bat.muted') : tr('bat.active'); $('mic-status').className = device.muted ? 'status-value inactive' : 'status-value';
  $('signal-value').textContent = `${Number(device.signal_dbm ?? 0)} dBm`; $('sidetone').checked = !!device.sidetone;
  $('mic-toggle').textContent = device.muted ? tr('mic.off') : tr('mic.on'); $('mic-toggle').classList.toggle('active', !!device.muted);
  setConnection('connected', pct); $('device-message').className = 'device-message connected'; $('device-message-title').textContent = tr('dash.connected_title'); $('device-message-text').textContent = charging ? tr('dash.charging_text') : tr('dash.connected_text');
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
        opt.textContent = `${d.name || 'Гарнитура #' + (idx+1)} (${d.battery_percent}%)`;
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
  try { await invoke('check_battery_voice'); toast(tr('toast.battery_voice_played')); }
  catch (error) { console.error('check_battery_voice failed', error); toast(`${tr('toast.voice_check_error')}: ${error}`); }
}

function rgbaToHex(a) { return `#${[0,1,2].map(i => Number(a?.[i] ?? 0).toString(16).padStart(2,'0')).join('')}`; }
function hexToRgba(hex, alpha = 255) { const h = String(hex || '#000000').replace('#',''); return [parseInt(h.slice(0,2),16)||0, parseInt(h.slice(2,4),16)||0, parseInt(h.slice(4,6),16)||0, alpha]; }
function setColor(id, value) { const el = $(id); if (el) el.value = rgbaToHex(value); }
function colorValue(id, fallback) { return hexToRgba($(id)?.value || '#000000', fallback?.[3] ?? 255); }

function trayToUi(t) {
  trayConfig = structuredClone(t);
  $('tray-mode').value = t.mode || 'big';
  const ds = $('digits-settings'); if (ds) ds.style.display = $('tray-mode').value === 'big' ? 'none' : '';
  $('tray-size').value = t.size; $('tray-font-scale').value = t.font_scale; $('tray-outline').value = t.outline_width; $('tray-border').value = t.border_width; $('tray-gap').value = t.gap_between_digits;
  for (const name of ['charging','high','medium','low']) {
    setColor(`tray-${name}-bg`, t.colors[name].bg); setColor(`tray-${name}-fg`, t.colors[name].fg); setColor(`tray-${name}-outline`, t.colors[name].outline); setColor(`tray-${name}-border`, t.colors[name].border);
  }
}
function uiToTray() {
  const t = structuredClone(trayConfig || {});
  t.mode = $('tray-mode').value || 'big';
  t.size = Number($('tray-size').value) || 256; t.font_scale = Number($('tray-font-scale').value) || 8; t.outline_width = Number($('tray-outline').value) || 0; t.border_width = Number($('tray-border').value) || 0; t.gap_between_digits = Number($('tray-gap').value) || 0;
  if (!t.colors) return t;
  for (const name of ['charging','high','medium','low']) {
    t.colors[name].bg = colorValue(`tray-${name}-bg`, t.colors[name].bg); t.colors[name].fg = colorValue(`tray-${name}-fg`, t.colors[name].fg); t.colors[name].outline = colorValue(`tray-${name}-outline`, t.colors[name].outline); t.colors[name].border = colorValue(`tray-${name}-border`, t.colors[name].border);
  }
  return t;
}

const DIGITS = [
  [0b01110,0b10001,0b10011,0b10101,0b11001,0b10001,0b01110],
  [0b00100,0b01100,0b00100,0b00100,0b00100,0b00100,0b01110],
  [0b01110,0b10001,0b00001,0b00010,0b00100,0b01000,0b11111],
  [0b11111,0b00010,0b00100,0b00010,0b00001,0b10001,0b01110],
  [0b00010,0b00110,0b01010,0b10010,0b11111,0b00010,0b00010],
  [0b11111,0b10000,0b11110,0b00001,0b00001,0b10001,0b01110],
  [0b01110,0b10001,0b10000,0b11110,0b10001,0b10001,0b01110],
  [0b11111,0b00001,0b00010,0b00100,0b01000,0b01000,0b01000],
  [0b01110,0b10001,0b10001,0b01110,0b10001,0b10001,0b01110],
  [0b01110,0b10001,0b10001,0b01111,0b00001,0b10001,0b01110],
];

function rgbaToRgbStr(a) { return `rgb(${a[0]},${a[1]},${a[2]})`; }

function renderTrayPreview() {
  const canvas = $('tray-preview');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  const t = uiToTray();
  const pct = currentBattery;
  let schemeKey = pct <= 20 ? 'low' : pct <= 50 ? 'medium' : 'high';
  if (currentCharging) schemeKey = 'charging';
  const scheme = t.colors?.[schemeKey] || t.colors?.high || { bg:[0,180,80,255], fg:[255,255,255,255], outline:[10,10,10,255], border:[0,110,50,255] };
  const percent = pct;
  const sz = 64;
  canvas.width = sz; canvas.height = sz;
  ctx.clearRect(0, 0, sz, sz);

  if (t.mode === 'big') {
    const fg = scheme.fg, ol = scheme.fg;
    const text = String(percent);
    const n = text.length;
    const gap = 2;
    const maxDW = (sz - gap * (n - 1)) / (5 * n);
    const maxDH = sz / 7;
    const scale = Math.max(1, Math.min(Math.floor(maxDW), Math.floor(maxDH)));
    const dw = 5 * scale, dh = 7 * scale;
    const totalW = n * dw + (n - 1) * gap;
    const sx = Math.floor((sz - totalW) / 2);
    const sy = Math.floor((sz - dh) / 2);
    const opx = Math.max(1, Math.floor(scale / 3));

    for (let ci = 0; ci < n; ci++) {
      const d = parseInt(text[ci]);
      const digit = DIGITS[d];
      const ox = sx + ci * (dw + gap);
      for (let row = 0; row < 7; row++) {
        for (let col = 0; col < 5; col++) {
          if ((digit[row] >> (4 - col)) & 1) {
            for (let dy = -opx; dy <= opx; dy++) {
              for (let dx = -opx; dx <= opx; dx++) {
                const xi = ox + col * scale + dx, yi = sy + row * scale + dy;
                if (xi >= 0 && yi >= 0 && xi < sz && yi < sz) {
                  ctx.fillStyle = rgbaToRgbStr(ol); ctx.fillRect(xi, yi, 1, 1);
                }
              }
            }
            for (let dy = 0; dy < scale; dy++) {
              for (let dx = 0; dx < scale; dx++) {
                const x = ox + col * scale + dx, y = sy + row * scale + dy;
                if (x < sz && y < sz) { ctx.fillStyle = rgbaToRgbStr(fg); ctx.fillRect(x, y, 1, 1); }
              }
            }
          }
        }
      }
    }
  } else {
    const bg = scheme.bg, fg = scheme.fg, ol = scheme.fg, bd = scheme.border;
    ctx.fillStyle = rgbaToRgbStr(bg); ctx.fillRect(0, 0, sz, sz);
    const bw = t.border_width || 0;
    if (bw > 0) { ctx.fillStyle = rgbaToRgbStr(bd); ctx.fillRect(0, 0, sz, bw); ctx.fillRect(0, sz - bw, sz, bw); ctx.fillRect(0, 0, bw, sz); ctx.fillRect(sz - bw, 0, bw, sz); }
    const fs = t.font_scale || 8, opx = t.outline_width || 2, gap = t.gap_between_digits || 4;
    const dw = 5 * fs, dh = 7 * fs;
    const text = String(percent);
    const n = text.length;
    const totalW = n * dw + (n - 1) * gap;
    const sx = Math.floor((sz - totalW) / 2);
    const sy = Math.floor((sz - dh) / 2);

    for (let ci = 0; ci < n; ci++) {
      const d = parseInt(text[ci]);
      const digit = DIGITS[d];
      const ox = sx + ci * (dw + gap);
      if (opx > 0) {
        for (let row = 0; row < 7; row++) for (let col = 0; col < 5; col++) {
          if ((digit[row] >> (4 - col)) & 1) {
            for (let dy = -opx; dy <= opx; dy++) for (let dx = -opx; dx <= opx; dx++) {
              const xi = ox + col * fs + dx, yi = sy + row * fs + dy;
              if (xi >= 0 && yi >= 0 && xi < sz && yi < sz) { ctx.fillStyle = rgbaToRgbStr(ol); ctx.fillRect(xi, yi, 1, 1); }
            }
          }
        }
      }
      for (let row = 0; row < 7; row++) for (let col = 0; col < 5; col++) {
        if ((digit[row] >> (4 - col)) & 1) {
          for (let dy = 0; dy < fs; dy++) for (let dx = 0; dx < fs; dx++) {
            const x = ox + col * fs + dx, y = sy + row * fs + dy;
            if (x < sz && y < sz) { ctx.fillStyle = rgbaToRgbStr(fg); ctx.fillRect(x, y, 1, 1); }
          }
        }
      }
    }
  }
  const info = $('tray-preview-info');
  if (info) info.textContent = `${t.mode} · ${sz}×${sz} · scheme: ${schemeKey} (${percent}%)`;
}

['tray-mode','tray-size','tray-font-scale','tray-outline','tray-border','tray-gap',
 'tray-charging-bg','tray-charging-fg','tray-charging-outline','tray-charging-border',
 'tray-high-bg','tray-high-fg','tray-high-outline','tray-high-border',
 'tray-medium-bg','tray-medium-fg','tray-medium-outline','tray-medium-border',
 'tray-low-bg','tray-low-fg','tray-low-outline','tray-low-border'].forEach(id => {
  const el = $(id); if (el) { el.addEventListener('input', renderTrayPreview); el.addEventListener('change', renderTrayPreview); }
});

$('tray-mode').addEventListener('change', () => {
  const mode = $('tray-mode').value;
  const ds = $('digits-settings'); if (ds) ds.style.display = mode === 'big' ? 'none' : '';
  renderTrayPreview();
});

function configToUi(c) {
  config = structuredClone(c);
  $('cfg-enabled').checked = !!c.enabled; $('cfg-sidetone').checked = !!c.device?.sidetone; $('cfg-voice-prompts').checked = !!c.device?.voice_prompts;
  $('cfg-auto-shutdown').value = c.device?.auto_shutdown_minutes ?? 30;
  const modeRaw = String(c.input?.mute_button_mode ?? 'smart_double');
  $('cfg-mute-mode').value = ['standard','media_play_pause','smart_double','smart_hold','hold_play_pause'].includes(modeRaw) ? modeRaw : 'smart_double';
  $('cfg-keybind').value = c.keybind ?? 'F20'; $('cfg-double-tap').value = c.double_tap_ms ?? 500;
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
    const [c, t, autoStart] = await Promise.all([invoke('get_config'), invoke('get_tray_config'), invoke('get_autostart_enabled')]);
    const lang = c.language || 'ru';
    window.__currentLang = lang;
    document.documentElement.lang = lang;
    if (typeof window.applyLanguage === 'function') window.applyLanguage(lang);
    configToUi(c); trayToUi(t); renderTrayPreview();
    $('cfg-start-os').checked = autoStart;
    if (c.start_in_compact_mode) setTimeout(() => invoke('open_compact_window').catch(e => console.debug('compact startup:', e)), 250);
  }
  catch (error) { console.error('Settings load failed', error); toast(`${tr('toast.load_error')}: ${error}`); }
}

async function saveSettings() {
  const newConfig = uiToConfig(), newTray = uiToTray();
  try {
    await invoke('save_config', { config: newConfig }); config = newConfig;
    await invoke('save_tray_config', { config: newTray }); trayConfig = newTray;
    try { await invoke('apply_eq', { bands: newConfig.audio.eq_bands }); }
    catch (eqError) { console.info('EQ apply:', eqError); }
    await command('set_sidetone', { enabled: newConfig.device.sidetone }).catch(() => {});
    markSaved(); toast(tr('toast.settings_saved'));
  } catch (error) { console.error('Save failed', error); toast(`${tr('toast.save_error')}: ${error}`); }
}

$('btn-mute').addEventListener('click', () => command('toggle_mute'));
$('btn-reconnect').addEventListener('click', refresh);
$('btn-voice-check').addEventListener('click', checkBatteryVoice);
$('btn-test-voice').addEventListener('click', async () => { try { await invoke('test_voice'); toast(tr('toast.voice_test')); } catch (e) { toast(`${tr('toast.voice_error')}: ${e}`); } });
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
      toast(`${tr('toast.device_switch_error')}: ${err}`);
    }
  });
}

document.querySelectorAll('#settings input, #settings select').forEach(el => { el.addEventListener('change', markDirty); el.addEventListener('input', markDirty); });

$('cfg-start-os').addEventListener('change', async (e) => {
  try { await invoke('set_autostart_enabled', { enabled: e.target.checked }); toast(e.target.checked ? tr('toast.autostart_on') : tr('toast.autostart_off')); }
  catch (err) { toast(`Autostart: ${err}`); }
});

$('cfg-language').addEventListener('change', (e) => {
  const lang = e.target.value;
  window.__currentLang = lang;
  document.documentElement.lang = lang;
  if (config) config.language = lang;
  if (typeof window.applyLanguage === 'function') window.applyLanguage(lang);
  markDirty();
});

for (const button of document.querySelectorAll('.tab-btn')) button.addEventListener('click', () => { document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active')); document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active')); button.classList.add('active'); $(button.dataset.tab).classList.add('active'); });
for (const button of document.querySelectorAll('.settings-tab')) button.addEventListener('click', () => { document.querySelectorAll('.settings-tab').forEach(b => b.classList.remove('active')); document.querySelectorAll('.settings-pane').forEach(p => p.classList.remove('active')); button.classList.add('active'); $(button.dataset.settingsTab).classList.add('active'); });

const presets = { 'Плоский':[0,0,0,0,0,0,0,0,0,0], 'Усиление басов':[6,4,2,0,0,0,0,0,0,0], 'Ослабление басов':[-6,-4,-2,0,0,0,0,0,0,0], 'Усиление верхних':[0,0,0,0,0,0,2,4,6,8], 'Голосовой чат':[-2,0,2,4,6,6,4,2,0,-2], 'Игры':[4,3,2,1,0,0,1,2,3,4] };
for (const band of document.querySelectorAll('.eq-band input')) band.addEventListener('input', markDirty);
$('eq-reset').addEventListener('click', () => { document.querySelectorAll('.eq-band input').forEach(i => i.value = 0); markDirty(); });
$('eq-preset').addEventListener('change', () => { const values = presets[$('eq-preset').value] || presets.Flat; document.querySelectorAll('.eq-band input').forEach((input, i) => input.value = values[i]); markDirty(); });
$('eq-apply').addEventListener('click', async () => {     const bands = Array.from(document.querySelectorAll('.eq-band input')).map(i => Number(i.value)); try { await invoke('apply_eq', { bands }); toast(tr('toast.eq_applied')); } catch (e) { toast(`${tr('toast.eq_applied')}: ${e}`); } });

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
      ? `${tr('eq.active_for')} "${per.name || 'this headset'}"`
      : `${tr('eq.using_global')} "${st.name || 'this headset'}"`;
    btnClear.style.display = per && per.audio ? 'inline-block' : 'none';
  } catch (e) { console.debug('per-device eq:', e); }
}
$('eq-save-per-device').addEventListener('click', async () => {
  if (!activeDeviceId) { toast(tr('toast.no_device')); return; }
  try {
    const bands = Array.from(document.querySelectorAll('.eq-band input')).map(i => Number(i.value));
    const st = await invoke('get_device_state');
    let per = null;
    try { per = await invoke('get_per_device_config', { deviceId: activeDeviceId }); } catch (_) {}
    per = Object.assign({ name: '', audio: null, device: null, voice: null }, per || {});
    per.name = st.name || per.name;
    per.audio = { eq_bands: bands };
    await invoke('save_per_device_config', { deviceId: activeDeviceId, perCfg: per });
    toast(tr('toast.eq_saved')); refreshPerDeviceEq();
  } catch (e) { toast(`${tr('toast.save_error')}: ${e}`); }
});
$('eq-clear-per-device').addEventListener('click', async () => {
  if (!activeDeviceId) return;
  try {
    let per = await invoke('get_per_device_config', { deviceId: activeDeviceId });
    if (per) { per.audio = null; await invoke('save_per_device_config', { deviceId: activeDeviceId, perCfg: per }); }
    toast(tr('toast.eq_removed')); refreshPerDeviceEq();
  } catch (e) { toast(`${tr('toast.save_error')}: ${e}`); }
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
  try { await invoke('set_custom_voice_dir', { path: $('voice-custom-dir').value.trim() }); toast(tr('toast.voice_folder_saved')); }
  catch (e) { toast(`${tr('toast.save_error')}: ${e}`); }
});
const voiceUpload = $('voice-upload-file');
if (voiceUpload) voiceUpload.addEventListener('change', async (e) => {
  const file = e.target.files[0];
  if (!file) return;
  try {
    const buf = new Uint8Array(await file.arrayBuffer());
    const saved = await invoke('upload_voice_file', { filename: file.name, data: Array.from(buf) });
    toast(`${tr('toast.uploaded')}: ${saved}`);
  } catch (err) { toast(`${tr('toast.upload_error')}: ${err}`); }
  e.target.value = '';
});
loadVoiceDir();

const btnUpdates = $('btn-check-updates');
if (btnUpdates) btnUpdates.addEventListener('click', async () => {
  try {
    const updater = window.__TAURI__.updater;
    const relaunch = window.__TAURI__.process ? window.__TAURI__.process.relaunch : null;
    if (!updater || !updater.check) { toast(tr('toast.unsupported')); return; }
    toast(tr('toast.update_checking'));
    const update = await updater.check();
    if (!update) { toast(tr('toast.up_to_date')); return; }
    toast(`${tr('toast.downloading')} ${update.version}...`);
    await update.downloadAndInstall();
    if (relaunch) await relaunch();
  } catch (e) {
    console.error('update check failed:', e);
    toast(`${tr('toast.update_error')}: ${e}`);
  }
});

listen('device-state', event => updateBattery(event.payload));
listen('device-connected', refresh);
listen('device-disconnected', () => updateBattery({ connected:false }));
listen('device-command-error', event => toast(`${tr('toast.device_cmd_error')}: ${event.payload}`));
listen('device-command-ok', event => { if (event.payload === 'mute') toast(tr('toast.mute_toggled')); });

loadConfig();
refresh();
refreshAudio();
setInterval(refresh, 2000);
setInterval(refreshAudio, 2000);
