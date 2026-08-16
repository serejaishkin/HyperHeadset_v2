const { invoke, listen } = window.__TAURI__.core;

async function toggleMute() { await invoke('toggle_mute'); }

function updateBattery(state) {
  const pct = document.getElementById('battery-percent');
  const status = document.getElementById('battery-status');
  const ring = document.getElementById('ring-progress');
  if (!state.connected) {
    pct.textContent = '--';
    status.textContent = 'Нет подключения';
    ring.style.strokeDashoffset = 283;
    return;
  }
  pct.textContent = state.battery_percent + '%';
  status.textContent = state.charging ? '⚡ Заряжается' : '🔋 Батарея';
  const offset = 283 - (283 * state.battery_percent / 100);
  ring.style.strokeDashoffset = offset;
}

document.getElementById('btn-mute').addEventListener('click', toggleMute);
document.getElementById('btn-check').addEventListener('click', () => invoke('get_device_state').then(updateBattery));

listen('device-state', e => updateBattery(e.payload));
listen('device-connected', () => { document.getElementById('battery-status').textContent = 'Подключено'; });
listen('device-disconnected', () => { document.getElementById('battery-status').textContent = 'Отключено'; });

invoke('get_device_state').then(updateBattery);

document.querySelectorAll('.tab-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById(btn.dataset.tab).classList.add('active');
  });
});
