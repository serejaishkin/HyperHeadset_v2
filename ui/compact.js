const { invoke } = window.__TAURI__.core;
const listen = window.__TAURI__.event ? window.__TAURI__.event.listen : window.__TAURI__.core.listen;

const $ = (id) => document.getElementById(id);
function tr(key, fb) { const lang = window.__currentLang || 'ru'; return window.I18N?.[lang]?.[key] ?? window.I18N?.ru?.[key] ?? fb ?? key; }

let sidetoneOn = false;

function updateCompact(state) {
    const connected = !!state?.connected;
    const pct = Math.max(0, Math.min(100, Number(state?.battery_percent ?? 0)));
    $('connection').className = `connection ${connected ? 'on' : ''}`;
    $('connection').title = connected ? tr('conn.on', 'ВКЛ') : tr('conn.off', 'ВЫКЛ');
    $('battery-pct').textContent = connected ? `${pct}%` : '--%';
    const bar = $('progress-bar'), status = $('battery-st'), mic = $('mic-status');
    if (!connected) {
        status.textContent = tr('bat.no_connection', 'Гарнитура выключена');
        bar.style.width = '0%'; bar.style.background = '#444';
        mic.textContent = '';
        sidetoneOn = false; $('sidetone-c').classList.remove('on');
        return;
    }
    status.textContent = (state.charging ? '⚡ ' + tr('bat.charging', 'Зарядка') : '🔋 ' + tr('bat.battery', 'Батарея')) + (state.name ? ' · ' + state.name : '');
    bar.style.width = `${pct}%`;
    bar.style.background = state.charging ? '#20e83a' : pct > 30 ? '#35d07f' : pct > 15 ? '#ff9800' : '#f44336';
    mic.textContent = state.muted ? '🔇 ' + tr('mic.off', 'Микрофон выкл') : '🎙 ' + tr('mic.on', 'Микрофон вкл');
    $('btn-mute-c').classList.toggle('on', !!state.muted);
    sidetoneOn = !!state.sidetone; $('sidetone-c').classList.toggle('on', sidetoneOn);
}

async function refreshCompact() {
    try { updateCompact(await invoke('get_device_state')); }
    catch (error) { console.error('get_device_state failed', error); }
}

$('btn-mute-c').addEventListener('click', async () => { try { await invoke('toggle_mute'); } catch (e) { console.error(e); } });
$('btn-main-c').addEventListener('click', async () => {
    try { await invoke('show_main_window_cmd'); } catch (e) { console.error(e); window.close(); }
});
$('sidetone-c').addEventListener('click', async () => {
    try { await invoke('set_sidetone', { enabled: !sidetoneOn }); sidetoneOn = !sidetoneOn; $('sidetone-c').classList.toggle('on', sidetoneOn); }
    catch (e) { console.error('set_sidetone failed', e); }
});

$('vol-master').addEventListener('input', async (e) => {
    $('vol-value').textContent = `${e.target.value}%`;
    try { await invoke('set_volume', { percent: Number(e.target.value) }); } catch (error) { console.debug(error); }
});
$('vol-mic').addEventListener('input', async (e) => {
    $('mic-value').textContent = `${e.target.value}%`;
    try { await invoke('set_mic_volume', { percent: Number(e.target.value) }); } catch (error) { console.debug(error); }
});

// ---- audio devices: show the CURRENT system defaults, switch on the fly
let devices = [];
function fill(select, list, current) {
    select.innerHTML = '';
    list.forEach(d => { const o = document.createElement('option'); o.value = d.id; o.textContent = d.name; select.appendChild(o); });
    if (current && list.some(d => d.id === current)) select.value = current;
    else if (!list.length) { const o = document.createElement('option'); o.textContent = '—'; select.appendChild(o); }
}
async function refreshDevices(force) {
    try {
        const [list, def] = await Promise.all([invoke('get_audio_devices'), invoke('get_default_audio_devices')]);
        devices = list;
        const o = $('out-c'), i = $('in-c');
        const sig = JSON.stringify([list.map(d => d.id), def]);
        if (!force && sig === refreshDevices.sig) return;
        refreshDevices.sig = sig;
        if (document.activeElement !== o) fill(o, list.filter(d => d.is_output), def.output);
        if (document.activeElement !== i) fill(i, list.filter(d => d.is_input), def.input);
    } catch (e) { console.error('refreshDevices failed', e); }
}
$('out-c').addEventListener('change', async (e) => {
    try { await invoke('set_default_output_device', { deviceId: e.target.value }); } catch (err) { console.error(err); }
    e.target.blur(); refreshDevices(true);
});
$('in-c').addEventListener('change', async (e) => {
    try { await invoke('set_default_input_device', { deviceId: e.target.value }); } catch (err) { console.error(err); }
    e.target.blur(); refreshDevices(true);
});

async function refreshLevels() {
    try {
        const l = await invoke('get_audio_levels');
        if (document.activeElement !== $('vol-master')) { $('vol-master').value = l.output; $('vol-value').textContent = `${l.output}%`; }
        if (document.activeElement !== $('vol-mic')) { $('vol-mic').value = l.input; $('mic-value').textContent = `${l.input}%`; }
    } catch (e) { /* not available */ }
}

// ---- auto-switch toggle (config)
async function loadConfig() {
    try {
        const c = await invoke('get_config');
        const lang = c.language || 'ru';
        window.__currentLang = lang;
        document.documentElement.lang = lang;
        $('auto-switch-c').classList.toggle('on', !!c.audio?.auto_switch_audio);
    } catch (e) { console.error('loadConfig failed', e); }
}
$('auto-switch-c').addEventListener('click', async () => {
    try {
        const c = await invoke('get_config');
        c.audio.auto_switch_audio = !c.audio.auto_switch_audio;
        await invoke('save_config', { config: c });
        $('auto-switch-c').classList.toggle('on', !!c.audio.auto_switch_audio);
    } catch (e) { console.error('auto switch failed', e); }
});

listen('device-state', e => updateCompact(e.payload));
listen('device-disconnected', () => updateCompact({ connected: false }));
listen('device-command-error', e => console.error('Device command failed:', e.payload));

refreshCompact(); loadConfig(); refreshDevices(true); refreshLevels();
setInterval(refreshCompact, 2000);
setInterval(() => refreshDevices(false), 3000);
setInterval(refreshLevels, 3000);
