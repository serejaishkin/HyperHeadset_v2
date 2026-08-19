const { invoke, listen } = window.__TAURI__.core;

const $ = (id) => document.getElementById(id);
let toastTimer = null;

function toast(message) {
  let node = $('toast');
  if (!node) {
    node = document.createElement('div');
    node.id = 'toast';
    node.className = 'toast';
    document.body.appendChild(node);
  }
  node.textContent = message;
  node.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => node.classList.remove('show'), 2500);
}

function setConnection(mode, battery = null) {
  const state = $('connection-state');
  state.className = `connection-state ${mode}`;
  state.textContent = mode === 'connected' ? 'ON' : mode === 'searching' ? 'SEARCHING' : 'OFF';
  $('header-battery').textContent = battery == null ? '[BAT] --%' : `[BAT] ${battery}%`;
  $('settings-connection').textContent = mode === 'connected' ? 'Connected' : mode === 'searching' ? 'Searching...' : 'Disconnected';
}

function updateBattery(device) {
  const connected = !!device?.connected;
  const percent = Number(device?.battery_percent ?? 0);
  const charging = !!device?.charging;
  const fill = $('battery-bar-fill');

  if (!connected) {
    $('battery-percent').textContent = '--%';
    $('battery-status').textContent = 'No connection';
    $('mic-status').textContent = 'Inactive';
    $('mic-status').className = 'status-value inactive';
    $('signal-value').textContent = '-- dBm';
    fill.style.width = '0%';
    fill.className = '';
    setConnection('disconnected');
    $('settings-battery').textContent = '--%';
    $('settings-charging').textContent = 'Unknown';
    $('settings-mic').textContent = 'Unknown';
    $('device-message').className = 'device-message';
    $('device-message-title').textContent = 'Headset disconnected';
    $('device-message-text').textContent = 'Waiting for the device...';
    return;
  }

  const pct = Math.max(0, Math.min(100, percent));
  $('battery-percent').textContent = `${pct}%`;
  $('battery-status').textContent = charging ? 'Charging' : 'Battery';
  fill.style.width = `${pct}%`;
  fill.className = charging ? 'charging' : pct <= 20 ? 'low' : '';
  $('mic-status').textContent = device.muted ? 'Muted' : 'Active';
  $('mic-status').className = device.muted ? 'status-value inactive' : 'status-value';
  $('signal-value').textContent = `${Number(device.signal_dbm ?? 0)} dBm`;
  $('sidetone').checked = !!device.sidetone;
  $('mic-toggle').textContent = device.muted ? 'MIC OFF' : 'MIC ON';
  $('mic-toggle').classList.toggle('active', !!device.muted);
  setConnection('connected', pct);
  $('settings-battery').textContent = `${pct}%`;
  $('settings-charging').textContent = charging ? 'Charging' : 'Not charging';
  $('settings-mic').textContent = device.muted ? 'Muted' : 'Active';
  $('device-message').className = 'device-message connected';
  $('device-message-title').textContent = 'Headset connected';
  $('device-message-text').textContent = charging ? 'Charging' : 'Device is ready';
}

async function refresh() {
  try {
    updateBattery(await invoke('get_device_state'));
  } catch (error) {
    console.error('get_device_state failed', error);
  }
}

async function command(name, args = {}) {
  try {
    await invoke(name, args);
  } catch (error) {
    console.error(`${name} failed`, error);
    toast(`${name}: ${error}`);
  }
}

$('btn-mute').addEventListener('click', () => command('toggle_mute'));
$('btn-check').addEventListener('click', refresh);
$('btn-reconnect').addEventListener('click', refresh);
$('btn-compact').addEventListener('click', async () => {
  await command('open_compact_window');
});
$('sidetone').addEventListener('change', (e) => command('set_sidetone', { enabled: e.target.checked }));
$('voice-prompts').addEventListener('change', (e) => command('set_voice_prompts', { enabled: e.target.checked }));
$('volume').addEventListener('input', (e) => $('volume-value').textContent = `${e.target.value}%`);
$('mic-volume').addEventListener('input', (e) => $('mic-value').textContent = `${e.target.value}%`);
$('mic-toggle').addEventListener('click', () => command('toggle_mute'));
$('play-button').addEventListener('click', () => toast('PLAY command is not connected to the Rust audio backend yet'));

for (const button of document.querySelectorAll('.tab-btn')) {
  button.addEventListener('click', () => {
    document.querySelectorAll('.tab-btn').forEach((b) => b.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach((c) => c.classList.remove('active'));
    button.classList.add('active');
    $(button.dataset.tab).classList.add('active');
  });
}

for (const button of document.querySelectorAll('.settings-tab')) {
  button.addEventListener('click', () => {
    document.querySelectorAll('.settings-tab').forEach((b) => b.classList.remove('active'));
    document.querySelectorAll('.settings-pane').forEach((p) => p.classList.remove('active'));
    button.classList.add('active');
    $(button.dataset.settingsTab).classList.add('active');
  });
}

for (const band of document.querySelectorAll('.eq-band input')) {
  band.addEventListener('input', () => {});
}
$('eq-reset').addEventListener('click', () => document.querySelectorAll('.eq-band input').forEach((input) => input.value = 0));
$('eq-preset').addEventListener('change', () => {
  const preset = $('eq-preset').value;
  const values = { Flat: [0,0,0,0,0,0,0], FPS: [2,1,-1,2,4,3,1], Music: [4,3,1,0,2,3,4], Movie: [3,2,0,1,2,3,2] };
  document.querySelectorAll('.eq-band input').forEach((input, i) => input.value = values[preset][i]);
});

listen('device-state', (event) => updateBattery(event.payload));
listen('device-connected', () => refresh());
listen('device-disconnected', () => updateBattery({ connected: false }));
listen('device-command-error', (event) => toast(`Device command failed: ${event.payload}`));
listen('device-command-ok', (event) => {
  if (event.payload === 'mute') toast('Mute state changed');
});

refresh();
setTimeout(refresh, 300);
