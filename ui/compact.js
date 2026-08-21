const { invoke, listen } = window.__TAURI__.core;

const $ = (id) => document.getElementById(id);

function updateCompact(state) {
    const connected = !!state?.connected;
    const pct = Math.max(0, Math.min(100, Number(state?.battery_percent ?? 0)));
    const connection = $('connection');
    const battery = $('battery-pct');
    const status = $('battery-st');
    const bar = $('progress-bar');
    const mic = $('mic-status');

    connection.textContent = connected ? 'ON' : 'OFF';
    connection.className = `connection ${connected ? 'on' : 'off'}`;
    battery.textContent = connected ? `${pct}%` : '--%';

    if (!connected) {
        status.textContent = 'Нет подключения';
        bar.style.width = '0%';
        bar.style.background = '#444';
        mic.textContent = '🎙️ MIC OFFLINE';
        return;
    }

    status.textContent = state.charging ? '⚡ Заряжается' : '🔋 Батарея';
    bar.style.width = `${pct}%`;
    bar.style.background = state.charging ? '#20e83a' : pct > 30 ? '#35d07f' : pct > 15 ? '#ff9800' : '#f44336';
    mic.textContent = state.muted ? '🔇 MUTE' : '🎙️ MIC ON';
}

async function refreshCompact() {
    try { updateCompact(await invoke('get_device_state')); }
    catch (error) { console.error('get_device_state failed', error); }
}

$('btn-mute-c').addEventListener('click', async () => {
    try { await invoke('toggle_mute'); }
    catch (error) { console.error('toggle_mute failed', error); }
});

$('btn-main-c').addEventListener('click', () => {
    // The Rust close handler turns a close request into "hide compact + show main".
    window.close();
});

$('vol-master').addEventListener('input', async (e) => {
    $('vol-value').textContent = `${e.target.value}%`;
    try { await invoke('set_volume', { percent: Number(e.target.value) }); } catch (error) { console.debug(error); }
});

$('vol-mic').addEventListener('input', async (e) => {
    $('mic-value').textContent = `${e.target.value}%`;
    try { await invoke('set_mic_volume', { percent: Number(e.target.value) }); } catch (error) { console.debug(error); }
});

listen('device-state', e => updateCompact(e.payload));
listen('device-disconnected', () => updateCompact({ connected:false }));
listen('device-command-error', e => console.error('Device command failed:', e.payload));

refreshCompact();
setInterval(refreshCompact, 1000);
