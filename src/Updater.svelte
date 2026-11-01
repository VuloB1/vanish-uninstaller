<script>
  import { fly } from 'svelte/transition';
  import { t } from './i18n.svelte.js';
  import { up, install } from './update.svelte.js';
</script>

{#if up.update && !up.dismissed}
  <div class="toast card" transition:fly={{ y: -10, duration: 220 }} role="status">
    {#if up.phase === 'idle'}
      <span class="msg">{t('up.available', { v: up.update.version })}</span>
      <button class="btn" onclick={() => (up.dismissed = true)}>{t('up.later')}</button>
      <button class="btn primary" onclick={install}>{t('up.now')}</button>
    {:else if up.phase === 'downloading'}
      <div class="spin"></div>
      <span class="msg">{t('up.downloading', { p: up.pct })}</span>
      <div class="bar"><i style="width:{up.pct}%"></i></div>
    {:else if up.phase === 'installing'}
      <div class="spin"></div>
      <span class="msg">{t('up.installing')}</span>
    {:else}
      <span class="msg">{t('up.failed', { e: up.err })}</span>
      <button class="btn" onclick={() => ((up.dismissed = true), (up.phase = 'idle'))}>{t('btn.close')}</button>
    {/if}
  </div>
{/if}

<style>
  .toast { position: fixed; top: 48px; right: 16px; z-index: 45; display: flex; align-items: center; gap: 12px; max-width: 460px; padding: 12px 14px; box-shadow: 0 14px 34px -10px var(--shadow); }
  .msg { font-size: 13.5px; font-weight: 600; flex: 1; min-width: 0; }
  .toast .btn { padding: 6px 14px; font-size: 13px; }
  .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 3px; border-radius: 0 0 14px 14px; overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--grad); transition: width 0.2s; }
</style>
