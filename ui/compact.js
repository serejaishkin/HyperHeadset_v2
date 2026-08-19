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
        mic.textContent = '🎙️ MIC OFFLINE';
        return;
    }
    pct.textContent = state.battery_percent + '%';
    st.textContent = state.charging ? '⚡ Заряжается' : '🔋 Батарея';
    bar.style.width = state.battery_percent + '%';
    bar.style.background = state.charging ? '#20e83a' : state.battery_percent > 30 ? '#d9b52b' : state.battery_percent > 15 ? '#ff9800' : '#f44336';
    mic.textContent = state.muted ? '🔇 MUTE' : '🎙️ MIC ON';
}

document.getElementById('btn-mute-c').addEventListener('click', async () => {
    try {
        await invoke('toggle_mute');
    } catch (error) {
        console.error('toggle_mute failed', error);
    }
});

listen('device-state', e => updateCompact(e.payload));
listen('device-command-error', e => console.error('Device command failed:', e.payload));
invoke('get_device_state').then(updateCompact);
