<script>
  import { fly } from 'svelte/transition';
  import { check } from '@tauri-apps/plugin-updater';
  import { t } from './i18n.svelte.js';

  let update = $state(null);
  let phase = $state('idle'); // idle | downloading | installing | error
  let pct = $state(0);
  let err = $state('');

  // Looks for a newer release a few seconds after launch so startup stays instant. Offline or no release: stay silent.
  setTimeout(async () => {
    try {
      update = await check();
    } catch {}
  }, 4000);

  async function install() {
    phase = 'downloading';
    let total = 0;
    let done = 0;
    try {
      await update.downloadAndInstall((e) => {
        if (e.event === 'Started') total = e.data.contentLength ?? 0;
        else if (e.event === 'Progress') {
          done += e.data.chunkLength;
          pct = total ? Math.min(100, Math.round((done / total) * 100)) : 0;
        } else if (e.event === 'Finished') phase = 'installing';
      });
      phase = 'installing'; // the installer takes over and restarts the app
    } catch (e) {
      err = String(e);
      phase = 'error';
    }
  }
</script>

{#if update}
  <div class="toast card" transition:fly={{ y: -10, duration: 220 }} role="status">
    {#if phase === 'idle'}
      <span class="msg">{t('up.available', { v: update.version })}</span>
      <button class="btn" onclick={() => (update = null)}>{t('up.later')}</button>
      <button class="btn primary" onclick={install}>{t('up.now')}</button>
    {:else if phase === 'downloading'}
      <div class="spin"></div>
      <span class="msg">{t('up.downloading', { p: pct })}</span>
      <div class="bar"><i style="width:{pct}%"></i></div>
    {:else if phase === 'installing'}
      <div class="spin"></div>
      <span class="msg">{t('up.installing')}</span>
    {:else}
      <span class="msg">{t('up.failed', { e: err })}</span>
      <button class="btn" onclick={() => (update = null)}>{t('btn.close')}</button>
    {/if}
  </div>
{/if}

<style>
  .toast { position: fixed; top: 48px; right: 16px; z-index: 25; display: flex; align-items: center; gap: 12px; max-width: 460px; padding: 12px 14px; box-shadow: 0 14px 34px -10px var(--shadow); }
  .msg { font-size: 13.5px; font-weight: 600; flex: 1; min-width: 0; }
  .toast .btn { padding: 6px 14px; font-size: 13px; }
  .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 3px; border-radius: 0 0 14px 14px; overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--grad); transition: width 0.2s; }
</style>
