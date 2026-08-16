const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const EQ_FREQS = ["31Hz","63Hz","125Hz","250Hz","500Hz","1kHz","2kHz","4kHz","8kHz","16kHz"];

async function loadConfig() {
  const cfg = await invoke('get_config');
  document.getElementById('sidetone').checked = cfg.device.sidetone;
  document.getElementById('voice-prompts').checked = cfg.device.voice_prompts;
  document.getElementById('auto-shutdown').value = cfg.device.auto_shutdown_minutes;
  document.getElementById('mute-mode').value = cfg.input.mute_button_mode;
  document.getElementById('voice-enabled').checked = cfg.voice.enabled;
  document.getElementById('voice-battery-low').checked = cfg.voice.on_battery_low;
  document.getElementById('voice-charging').checked = cfg.voice.on_charging;
  document.getElementById('voice-full').checked = cfg.voice.on_full_charge;
  document.getElementById('voice-connected').checked = cfg.voice.on_connected;
  document.getElementById('voice-disconnected').checked = cfg.voice.on_disconnected;
  document.getElementById('voice-exact').checked = cfg.voice.exact_percent;
  document.getElementById('discord-appid').value = cfg.discord.direct.app_id;
  document.getElementById('discord-keybind').value = cfg.discord.keybind || 'F20';
  updateEQSliders(cfg.audio.eq_bands);
}

function updateEQSliders(bands) {
  const container = document.getElementById('eq-sliders');
  container.innerHTML = '';
  bands.forEach((val, i) => {
    const div = document.createElement('div');
    div.className = 'eq-band';
    div.innerHTML = `<label>${EQ_FREQS[i]}</label><input type="range" min="-12" max="12" step="0.1" value="${val}" data-idx="${i}"><span class="eq-val">${val.toFixed(1)}dB</span>`;
    container.appendChild(div);
  });
  container.querySelectorAll('input[type="range"]').forEach(sl => {
    sl.addEventListener('input', (e) => { e.target.nextElementSibling.textContent = parseFloat(e.target.value).toFixed(1) + 'dB'; });
  });
}

async function updateUI(state) {
  const pct = state.battery_percent || 0;
  const ring = document.getElementById('ring-progress');
  ring.style.strokeDashoffset = 283 - (283 * pct / 100);
  let color = '#4ade80'; if (pct <= 20) color = '#ef4444'; else if (pct <= 50) color = '#f59e0b';
  if (state.charging) color = '#facc15'; ring.style.stroke = color;
  document.getElementById('battery-percent').textContent = state.connected ? `${pct}%` : '--';
  document.getElementById('battery-status').textContent = state.connected ? (state.charging ? '⚡ Заряжается' : '🔋 Работает от батареи') : 'Нет подключения';
  document.getElementById('btn-mute').textContent = state.muted ? '🔇 Размьютить' : '🎙️ Мьют';
}

document.querySelectorAll('.tab-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
    btn.classList.add('active'); document.getElementById(btn.dataset.tab).classList.add('active');
  });
});
document.querySelectorAll('.sub-tab-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    const parent = btn.closest('main');
    parent.querySelectorAll('.sub-tab-btn').forEach(b => b.classList.remove('active'));
    parent.querySelectorAll('.sub-tab-content').forEach(c => c.classList.remove('active'));
    btn.classList.add('active'); parent.querySelector(`#${btn.dataset.subtab}`).classList.add('active');
  });
});
document.getElementById('btn-mute').addEventListener('click', () => invoke('toggle_mute'));
document.getElementById('btn-check').addEventListener('click', async () => updateUI(await invoke('get_device_state')));
document.querySelectorAll('.eq-presets button').forEach(btn => {
  btn.addEventListener('click', async () => { await invoke('apply_eq_preset', { preset: btn.dataset.preset }); updateEQSliders((await invoke('get_config')).audio.eq_bands); });
});
document.getElementById('btn-save-settings').addEventListener('click', async () => {
  const cfg = await invoke('get_config');
  cfg.device.sidetone = document.getElementById('sidetone').checked;
  cfg.device.voice_prompts = document.getElementById('voice-prompts').checked;
  cfg.device.auto_shutdown_minutes = parseInt(document.getElementById('auto-shutdown').value);
  cfg.input.mute_button_mode = document.getElementById('mute-mode').value;
  cfg.voice.enabled = document.getElementById('voice-enabled').checked;
  cfg.voice.on_battery_low = document.getElementById('voice-battery-low').checked;
  cfg.voice.on_charging = document.getElementById('voice-charging').checked;
  cfg.voice.on_full_charge = document.getElementById('voice-full').checked;
  cfg.voice.on_connected = document.getElementById('voice-connected').checked;
  cfg.voice.on_disconnected = document.getElementById('voice-disconnected').checked;
  cfg.voice.exact_percent = document.getElementById('voice-exact').checked;
  cfg.discord.direct.app_id = document.getElementById('discord-appid').value;
  cfg.discord.keybind = document.getElementById('discord-keybind').value;
  const bands = []; document.querySelectorAll('#eq-sliders input').forEach((sl, i) => { bands[i] = parseFloat(sl.value); });
  cfg.audio.eq_bands = bands;
  await invoke('save_config', { newConfig: cfg });
  alert('Сохранено!');
});
listen('device-state', (event) => updateUI(event.payload));
listen('device-connected', () => { document.getElementById('battery-status').textContent = 'Подключено'; });
listen('device-disconnected', () => { document.getElementById('battery-status').textContent = 'Отключено'; updateUI({ connected: false, battery_percent: 0, charging: false, muted: false }); });
loadConfig(); invoke('get_device_state').then(updateUI);