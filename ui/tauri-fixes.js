// Tauri UI synchronization fixes: one source of truth for settings + tray preview.
(() => {
  const { invoke } = window.__TAURI__.core;
  const $ = (id) => document.getElementById(id);
  let savedConfigSnapshot = null;
  let savedTraySnapshot = null;
  const clone = (v) => v == null ? v : structuredClone(v);
  const snapshot = (v) => JSON.stringify(v);

  function setSavedState() {
    savedConfigSnapshot = snapshot(window.__hhCurrentConfig || null);
    savedTraySnapshot = snapshot(window.__hhCurrentTray || null);
    if ($('settings-dirty')) {
      $('settings-dirty').textContent = 'Saved';
      $('settings-dirty').classList.remove('dirty');
    }
    if ($('save-message')) $('save-message').textContent = 'Changes are saved to config.toml and tray_icon.toml';
  }

  function setDirtyState() {
    if ($('settings-dirty')) {
      $('settings-dirty').textContent = 'Unsaved changes';
      $('settings-dirty').classList.add('dirty');
    }
    if ($('save-message')) $('save-message').textContent = 'There are unsaved changes';
  }

  function ensureTrayPreview() {
    const pane = $('tray-settings');
    if (!pane || $('tray-preview')) return;
    const box = document.createElement('div');
    box.id = 'tray-preview';
    box.innerHTML = '<div class="settings-section-title">Preview</div><div class="tray-preview-card"><div class="tray-preview-icon" id="tray-preview-icon">72%</div><div class="tray-preview-meta"><strong id="tray-preview-label">Battery 72%</strong><span>Live preview of the tray battery icon</span></div></div>';
    const title = pane.querySelector('.settings-section-title');
    if (title) title.insertAdjacentElement('afterend', box); else pane.prepend(box);
  }

  function color(id, fallback) { return $(id)?.value || fallback || '#000000'; }

  function updateTrayPreview() {
    ensureTrayPreview();
    const icon = $('tray-preview-icon');
    const label = $('tray-preview-label');
    if (!icon) return;
    const size = Math.max(16, Number($('tray-size')?.value || 256));
    const scale = Math.max(1, Number($('tray-font-scale')?.value || 8));
    const outline = Math.max(0, Number($('tray-outline')?.value || 0));
    const border = Math.max(0, Number($('tray-border')?.value || 0));
    const gap = Math.max(0, Number($('tray-gap')?.value || 0));
    const bg = color('tray-high-bg', '#00b450');
    const fg = color('tray-high-fg', '#ffffff');
    const out = color('tray-high-outline', '#0a0a0a');
    const bord = color('tray-high-border', '#006e32');
    const previewScale = Math.max(1, Math.min(5, scale / 3));
    icon.style.width = `${Math.max(80, Math.min(280, size / 2))}px`;
    icon.style.height = `${Math.max(56, Math.min(160, size / 2))}px`;
    icon.style.background = bg;
    icon.style.color = fg;
    icon.style.border = `${Math.max(0, Math.min(6, border / 2))}px solid ${bord}`;
    icon.style.textShadow = outline ? `0 0 ${Math.max(1, Math.min(8, outline / 2))}px ${out}` : 'none';
    icon.style.fontSize = `${Math.max(22, Math.min(72, 18 * previewScale))}px`;
    icon.style.letterSpacing = `${Math.min(20, gap)}px`;
    icon.textContent = '72%';
    if (label) label.textContent = `Battery 72% · ${size}px · scale ${scale} · gap ${gap}`;
  }

  async function syncSettings(force = false) {
    // Never overwrite fields while the user has unsaved edits.
    if (!force && $('settings-dirty')?.classList.contains('dirty')) return;
    try {
      const c = await invoke('get_config');
      const t = await invoke('get_tray_config');
      window.__hhCurrentConfig = clone(c);
      window.__hhCurrentTray = clone(t);
      if (typeof window.configToUi === 'function') window.configToUi(c);
      if (typeof window.trayToUi === 'function') window.trayToUi(t);
      setSavedState();
      updateTrayPreview();
    } catch (e) {
      console.error('[Settings] sync failed', e);
      updateTrayPreview();
    }
  }

  async function saveAll() {
    try {
      const c = typeof window.uiToConfig === 'function' ? window.uiToConfig() : null;
      const t = typeof window.uiToTray === 'function' ? window.uiToTray() : null;
      if (!c || !t) throw new Error('Settings form is not initialized');
      console.log('[HH] Saving tray config:', JSON.stringify(t?.colors, null, 2));
      await invoke('save_config', { config: c });
      await invoke('save_tray_config', { config: t });
      window.__hhCurrentConfig = clone(c);
      window.__hhCurrentTray = clone(t);
      try { await invoke('apply_eq', { bands: c.audio.eq_bands }); } catch (_) {}
      setSavedState();
      updateTrayPreview();
      if (typeof window.toast === 'function') window.toast('Settings saved');
    } catch (e) {
      console.error('[Settings] save failed', e);
      if (typeof window.toast === 'function') window.toast(`Save failed: ${e}`);
    }
  }

  function replaceButton(id, handler) {
    const old = $(id);
    if (!old || old.dataset.hhFixed) return;
    const fresh = old.cloneNode(true);
    fresh.dataset.hhFixed = '1';
    old.replaceWith(fresh);
    fresh.addEventListener('click', handler);
  }

  function wire() {
    ensureTrayPreview();
    replaceButton('btn-save-settings', saveAll);
    replaceButton('btn-reset-settings', () => syncSettings(true));
    document.querySelectorAll('#tray-settings input').forEach((el) => {
      el.addEventListener('input', () => { setDirtyState(); updateTrayPreview(); });
      el.addEventListener('change', () => { setDirtyState(); updateTrayPreview(); });
    });
    document.querySelectorAll('#settings input, #settings select').forEach((el) => {
      el.addEventListener('input', setDirtyState);
      el.addEventListener('change', setDirtyState);
    });
    setTimeout(() => syncSettings(true), 50);
    setInterval(() => syncSettings(false), 5000);
    setInterval(() => {
      if (!savedConfigSnapshot || !savedTraySnapshot || $('settings-dirty')?.classList.contains('dirty')) return;
      try {
        const currentC = typeof window.uiToConfig === 'function' ? window.uiToConfig() : null;
        const currentT = typeof window.uiToTray === 'function' ? window.uiToTray() : null;
        if (currentC && currentT && (snapshot(currentC) !== savedConfigSnapshot || snapshot(currentT) !== savedTraySnapshot)) setDirtyState();
      } catch (_) {}
    }, 1000);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', wire); else wire();
})();
