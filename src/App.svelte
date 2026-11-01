<script>
  import { api, win } from './api.js';
  import Programs from './Programs.svelte';
  import Junk from './Junk.svelte';
  import Registry from './Registry.svelte';
  import logo from './logo.svg';

  import Updater from './Updater.svelte';
  import Settings from './Settings.svelte';

  let showSettings = $state(false);
  import { t } from './i18n.svelte.js';

  const TABS = ['programs', 'junk', 'registry'];
  let tab = $state('programs');
  let admin = $state(true);
  api.admin().then((a) => (admin = a));
  // The webview's own menu (Reload, Inspect…) has no place in the app; text fields keep theirs for copy/paste.
  addEventListener('contextmenu', (e) => !e.target.closest?.('input, textarea') && e.preventDefault());
</script>

<div class="app">
  <div class="titlebar" data-tauri-drag-region>
    <div class="logo" data-tauri-drag-region><img src={logo} alt="" width="24" height="24" draggable="false" /><span><span class="grad">Van</span>ish <span class="sub">Uninstaller</span></span></div>
    <nav class="tabs">
      {#each TABS as id}
        <button class="tab" class:on={tab === id} onclick={() => (tab = id)}>{t(`app.tab.${id}`)}</button>
      {/each}
    </nav>
    <div class="spacer" data-tauri-drag-region></div>
    {#if !admin}
      <span class="adm" title={t('app.adminTip')}>{t('app.admin')}</span>
    {/if}
    <button class="tb" title={t('set.title')} aria-label={t('set.title')} onclick={() => (showSettings = true)}>
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg>
    </button>
    <span class="sep"></span>
    <button class="tb" aria-label={t('app.min')} onclick={() => win?.minimize()}>
      <svg viewBox="0 0 10 10" width="10" height="10"><path d="M0 5h10" stroke="currentColor" /></svg>
    </button>
    <button class="tb" aria-label={t('app.max')} onclick={() => win?.toggleMaximize()}>
      <svg viewBox="0 0 10 10" width="10" height="10" fill="none"><rect x="0.5" y="0.5" width="9" height="9" stroke="currentColor" /></svg>
    </button>
    <button class="tb close" aria-label={t('app.close')} onclick={() => win?.close()}>
      <svg viewBox="0 0 10 10" width="10" height="10"><path d="M0 0l10 10M10 0L0 10" stroke="currentColor" /></svg>
    </button>
  </div>

  {#if win}<Updater />{/if}
  <Settings bind:open={showSettings} />

  <div class="scroll">
    <main>
      <!-- All views stay mounted so switching tabs keeps their state and doesn't rescan. -->
      <div class="view" class:hide={tab !== 'programs'}><Programs /></div>
      <div class="view" class:hide={tab !== 'junk'}><Junk active={tab === 'junk'} /></div>
      <div class="view" class:hide={tab !== 'registry'}><Registry active={tab === 'registry'} /></div>
    </main>
  </div>
</div>

<style>
  .app { height: 100vh; display: flex; flex-direction: column; }
  .scroll { flex: 1; min-height: 0; overflow-y: auto; }
  .titlebar { position: relative; z-index: 40; flex: none; height: 38px; display: flex; align-items: center; gap: 4px; padding: 0 0 0 16px; background: var(--glass); backdrop-filter: blur(8px); border-bottom: 1px solid var(--border); user-select: none; }
  .logo { display: flex; align-items: center; gap: 8px; font-size: 17px; font-weight: 900; letter-spacing: -0.02em; }
  .logo .sub { font-weight: 600; font-size: 13px; color: var(--text-2); letter-spacing: 0; }
  .grad { background: var(--grad); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .spacer { flex: 1; align-self: stretch; }
  .tb { width: 42px; height: 38px; display: grid; place-items: center; border: 0; background: transparent; color: var(--text-2); }
  .tb:hover { background: var(--grad-soft); color: var(--text); }
  .tb.close:hover { background: #e81123; color: #fff; }
  .sep { width: 1px; height: 18px; background: var(--border); margin: 0 4px; }

  main { max-width: 980px; margin: 0 auto; padding: 24px 24px 40px; }
  .tabs { display: flex; gap: 2px; margin-left: 16px; }
  .tab { padding: 3px 11px; border-radius: 999px; border: 0; background: transparent; color: var(--text-2); font-size: 12.5px; font-weight: 600; transition: background 0.15s, color 0.15s; }
  .tab:hover { color: var(--text); background: var(--grad-soft); }
  .tab.on { background: var(--grad); color: #fff; }
  .adm { margin-right: 6px; padding: 1px 9px; border-radius: 999px; font-size: 11px; font-weight: 600; background: rgba(245, 158, 11, 0.14); border: 1px solid rgba(245, 158, 11, 0.45); cursor: help; }
</style>
