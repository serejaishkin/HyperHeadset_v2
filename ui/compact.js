const { invoke, listen } = window.__TAURI__.core;

function updateCompact(state) {
    const pct = document.getElementById('battery-pct');
    const st = document.getElementById('battery-st');
    const bar = document.getElementById('progress-bar');
    const mic = document.getElementById('mic-status');
    if (!state.connected) {
        pct.textContent = '--%';
        st.textContent = 'Нет подключения';
        bar.style.width = '0%';
        return;
    }
    pct.textContent = state.battery_percent + '%';
    st.textContent = state.charging ? '⚡ Заряжается' : '🔋 Батарея';
    bar.style.width = state.battery_percent + '%';
    bar.style.background = state.battery_percent > 30 ? '#4caf50' : state.battery_percent > 15 ? '#ff9800' : '#f44336';
    mic.textContent = state.muted ? '🔇 MUTE' : '🎙️ MIC ON';
}

document.getElementById('btn-mute-c').addEventListener('click', () => invoke('toggle_mute'));
listen('device-state', e => updateCompact(e.payload));
invoke('get_device_state').then(updateCompact);
