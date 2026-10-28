<script>
  import { fly } from 'svelte/transition';
  import { LANGS, getLang, setLang, t } from './i18n.svelte.js';

  let open = $state(false);
  let root;
</script>

<svelte:window
  onclick={(e) => open && root && !root.contains(e.target) && (open = false)}
  onkeydown={(e) => e.key === 'Escape' && (open = false)}
/>

<div class="lang" bind:this={root}>
  <button class="tb" class:open title={t('app.lang')} aria-label={t('app.lang')} aria-haspopup="menu" aria-expanded={open} onclick={() => (open = !open)}>
    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9" /><path d="M3 12h18M12 3c2.5 2.6 3.8 5.6 3.8 9S14.5 18.4 12 21c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3z" /></svg>
  </button>
  {#if open}
    <div class="menu card" role="menu" transition:fly={{ y: -6, duration: 150 }}>
      {#each LANGS as [code, name]}
        <button role="menuitemradio" aria-checked={code === getLang()} class:on={code === getLang()} onclick={() => (setLang(code), (open = false))}>
          {name}
          {#if code === getLang()}<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5 9-10" /></svg>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .lang { position: relative; }
  .tb { width: 42px; height: 38px; display: grid; place-items: center; border: 0; background: transparent; color: var(--text-2); }
  .tb:hover, .tb.open { background: var(--grad-soft); color: var(--text); }
  .menu { position: absolute; right: 0; top: calc(100% + 4px); z-index: 30; min-width: 150px; padding: 4px; box-shadow: 0 14px 30px -12px var(--shadow); }
  .menu button { display: flex; align-items: center; justify-content: space-between; gap: 12px; width: 100%; padding: 7px 12px; border: 0; border-radius: 8px; background: transparent; color: var(--text); font: inherit; font-size: 13px; text-align: left; transition: background 0.15s; }
  .menu button:hover { background: var(--grad-soft); }
  .menu button.on { font-weight: 700; }
  .menu button.on svg { color: var(--brand-b); }
</style>
