<script>
  import { fade, fly } from 'svelte/transition';
  import { api } from './api.js';
  import { t } from './i18n.svelte.js';
  import { PRESETS, getAccent, setAccent, colors } from './settings.svelte.js';
  import logo from './logo.svg';

  let { open = $bindable() } = $props();

  let version = $state('');
  api.version().then((v) => (version = v)).catch(() => {});

  const OWNER = 'Vulito';
  const PROFILE = 'https://github.com/VuloB1';
  const REPO = 'https://github.com/VuloB1/vanish-uninstaller';
  const custom = $derived(getAccent().id === 'custom' ? getAccent().b : '#0d9488');
</script>

<svelte:window onkeydown={(e) => open && e.key === 'Escape' && (open = false)} />

{#if open}
  <div class="overlay" transition:fade={{ duration: 160 }} onmousedown={(e) => e.target === e.currentTarget && (open = false)} role="presentation">
    <div class="card modal set" in:fly={{ y: 14, duration: 240 }} role="dialog" aria-label={t('set.title')}>
      <h2>{t('set.title')}</h2>

      <h3>{t('set.accent')}</h3>
      <div class="swatches">
        {#each PRESETS as [id, b, a]}
          <button class="sw" class:on={getAccent().id === id} title={t(`acc.${id}`)} aria-label={t(`acc.${id}`)} aria-pressed={getAccent().id === id} style="background: linear-gradient(135deg in oklab, {b}, {a})" onclick={() => setAccent({ id })}></button>
        {/each}
        <label class="sw custom" class:on={getAccent().id === 'custom'} title={t('set.custom')} style="background: linear-gradient(135deg in oklab, {colors({ id: 'custom', b: custom })[0]}, {colors({ id: 'custom', b: custom })[1]})">
          <input type="color" aria-label={t('set.custom')} value={custom} oninput={(e) => setAccent({ id: 'custom', b: e.currentTarget.value })} />
        </label>
        <button class="btn reset" onclick={() => setAccent({ id: 'teal' })}>{t('set.reset')}</button>
      </div>

      <h3>{t('set.about')}</h3>
      <div class="about">
        <img src={logo} alt="" width="56" height="56" draggable="false" />
        <div>
          <div class="name"><span class="grad">Vanish</span> Uninstaller</div>
          <div class="sub">{t('about.version', { v: version })}</div>
        </div>
      </div>
      <p class="sub">{t('about.desc')}</p>
      <div class="credits">
        <div>{t('about.by')} <button class="link" onclick={() => api.openLink(PROFILE)}>{OWNER}</button></div>
        <div><button class="link" onclick={() => api.openLink(REPO)}>{t('about.repo')}</button></div>
        <div class="sub">{t('about.license')}</div>
        <div class="sub">{t('about.built')}</div>
      </div>

      <footer><button class="btn primary" onclick={() => (open = false)}>{t('btn.close')}</button></footer>
    </div>
  </div>
{/if}

<style>
  .set { max-width: 480px; }
  .swatches { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; padding: 4px 0 6px; }
  .sw { position: relative; width: 34px; height: 34px; padding: 0; border: 0; border-radius: 50%; cursor: pointer; transition: transform 0.15s, box-shadow 0.15s; }
  .sw:hover { transform: scale(1.1); }
  .sw.on { box-shadow: 0 0 0 3px var(--card), 0 0 0 5px var(--text); }
  .custom { display: block; overflow: hidden; }
  .custom::after { content: '+'; position: absolute; inset: 0; display: grid; place-items: center; color: #fff; font-weight: 700; font-size: 18px; pointer-events: none; }
  .custom input { position: absolute; inset: -6px; width: 60px; height: 60px; opacity: 0; cursor: pointer; }
  .reset { padding: 6px 14px; font-size: 13px; margin-left: auto; }
  .about { display: flex; align-items: center; gap: 14px; margin-bottom: 8px; }
  .about .name { font-size: 18px; font-weight: 800; letter-spacing: -0.01em; }
  .grad { background: var(--grad); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .credits { display: grid; gap: 4px; margin-top: 8px; font-size: 13.5px; }
  .link { padding: 0; border: 0; background: none; color: var(--brand-b); font: inherit; font-weight: 700; cursor: pointer; text-decoration: underline; text-underline-offset: 3px; }
</style>
