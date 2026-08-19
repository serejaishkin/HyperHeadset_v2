const { invoke, listen } = window.__TAURI__.core;

const $ = (id) => document.getElementById(id);

function setConnection(mode, battery = null) {
  const state = $('connection-state');
  state.className = `connection-state ${mode}`;
  state.textContent = mode === 'connected' ? 'ON' : mode === 'searching' ? 'SEARCHING' : 'OFF';
  $('header-battery').textContent = battery == null ? '[BAT] --%' : `[BAT] ${battery}%`;
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
    fill.style.width = '0%';
    fill.className = '';
    setConnection('disconnected');
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
  $('sidetone').checked = !!device.sidetone;
  setConnection('connected', pct);
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
    await refresh();
  } catch (error) {
    console.error(`${name} failed`, error);
  }
}

$('btn-mute').addEventListener('click', () => command('toggle_mute'));
$('btn-check').addEventListener('click', refresh);
$('btn-reconnect').addEventListener('click', refresh);
$('btn-compact').addEventListener('click', () => command('open_compact_window'));
$('sidetone').addEventListener('change', (e) => command('set_sidetone', { enabled: e.target.checked }));
$('voice-prompts').addEventListener('change', (e) => command('set_voice_prompts', { enabled: e.target.checked }));

// Keep the legacy side controls visually present. Their device/audio backend is intentionally
// wired in a separate Tauri command layer instead of duplicating platform-specific Rust logic here.
$('volume').addEventListener('input', (e) => $('volume-value').textContent = `${e.target.value}%`);
$('mic-volume').addEventListener('input', (e) => $('mic-value').textContent = `${e.target.value}%`);
$('mic-toggle').addEventListener('click', () => command('toggle_mute'));
$('play-button').addEventListener('click', () => console.info('PLAY action is pending Tauri audio command'));

for (const button of document.querySelectorAll('.tab-btn')) {
  button.addEventListener('click', () => {
    document.querySelectorAll('.tab-btn').forEach((b) => b.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach((c) => c.classList.remove('active'));
    button.classList.add('active');
    $(button.dataset.tab).classList.add('active');
  });
}

listen('device-state', (event) => updateBattery(event.payload));
listen('device-connected', () => refresh());
listen('device-disconnected', () => updateBattery({ connected: false }));

refresh();
