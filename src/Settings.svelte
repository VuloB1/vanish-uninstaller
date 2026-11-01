<script>
  import { fade, fly, slide } from 'svelte/transition';
  import { api } from './api.js';
  import { t } from './i18n.svelte.js';
  import { PRESETS, getAccent, setAccent, colors, getLevel, setLevel } from './settings.svelte.js';
  import ColorPicker from './ColorPicker.svelte';
  import logo from './logo.svg';

  let { open = $bindable() } = $props();

  let version = $state('');
  api.version().then((v) => (version = v)).catch(() => {});

  const OWNER = 'Vulito';
  const PROFILE = 'https://github.com/VuloB1';
  const REPO = 'https://github.com/VuloB1/vanish-uninstaller';
  const custom = $derived(getAccent().id === 'custom' ? getAccent().b : '#0d9488');
  const grad = (c) => `linear-gradient(135deg in oklab, ${c[0]}, ${c[1]})`;
  let picking = $state(false);

  const reset = () => {
    setAccent({ id: 'teal' });
    setLevel(0);
    picking = false;
  };
</script>

<svelte:window onkeydown={(e) => open && e.key === 'Escape' && (open = false)} />

{#if open}
  <div class="overlay" transition:fade={{ duration: 160 }} onmousedown={(e) => e.target === e.currentTarget && (open = false)} role="presentation">
    <div class="card modal set" in:fly={{ y: 14, duration: 240 }} role="dialog" aria-label={t('set.title')}>
      <h2>{t('set.title')}</h2>

      <h3>{t('set.accent')}</h3>
      <div class="swatches">
        {#each PRESETS as [id, b, a]}
          <button class="sw" class:on={getAccent().id === id} title={t(`acc.${id}`)} aria-label={t(`acc.${id}`)} aria-pressed={getAccent().id === id} style="background: {grad([b, a])}" onclick={() => (setAccent({ id }), (picking = false))}></button>
        {/each}
        <button class="sw custom" class:on={getAccent().id === 'custom'} title={t('set.custom')} aria-label={t('set.custom')} aria-expanded={picking} style="background: {grad(colors({ id: 'custom', b: custom }))}" onclick={() => (picking = !picking)}>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="#fff" stroke-width="2.6" stroke-linecap="round"><path d={picking ? 'M6 15l6-6 6 6' : 'M12 5v14M5 12h14'} /></svg>
        </button>
      </div>
      {#if picking}
        <div transition:slide={{ duration: 200 }} class="picker"><ColorPicker value={custom} oninput={(b) => setAccent({ id: 'custom', b })} /></div>
      {/if}

      <h3>{t('set.tone')}</h3>
      <div class="tone">
        <span class="sub">{t('set.darker')}</span>
        <input type="range" min="-100" max="100" step="1" value={getLevel()} aria-label={t('set.tone')} oninput={(e) => setLevel(Number(e.currentTarget.value))} />
        <span class="sub">{t('set.lighter')}</span>
      </div>
      <button class="btn reset" onclick={reset}>{t('set.reset')}</button>

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
  .set { max-width: 480px; overflow-y: auto; }
  .swatches { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; padding: 4px 0 6px; }
  .sw { position: relative; display: grid; place-items: center; width: 34px; height: 34px; padding: 0; border: 0; border-radius: 50%; cursor: pointer; transition: transform 0.15s, box-shadow 0.15s; }
  .sw:hover { transform: scale(1.1); }
  .sw.on { box-shadow: 0 0 0 3px var(--card), 0 0 0 5px var(--text); }
  .picker { margin: 6px 0 4px; }
  .set > :global(*) { flex-shrink: 0; }

  .tone { display: flex; align-items: center; gap: 12px; }
  .tone .sub { white-space: nowrap; }
  .tone input { flex: 1; min-width: 0; height: 22px; margin: 0; background: transparent; appearance: none; cursor: pointer; }
  .tone input::-webkit-slider-runnable-track { height: 10px; border-radius: 999px; border: 1px solid var(--border); background: linear-gradient(to right, #050506, #77777b, #f4f4f2); }
  .tone input::-webkit-slider-thumb { appearance: none; width: 20px; height: 20px; margin-top: -6px; border-radius: 50%; border: 3px solid #fff; background: var(--brand-b); box-shadow: 0 1px 6px rgba(0, 0, 0, 0.4); }
  .reset { align-self: flex-start; margin-top: 10px; padding: 6px 14px; font-size: 13px; }

  .about { display: flex; align-items: center; gap: 14px; margin-bottom: 8px; }
  .about .name { font-size: 18px; font-weight: 800; letter-spacing: -0.01em; }
  .grad { background: var(--grad); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .credits { display: grid; gap: 4px; margin-top: 8px; font-size: 13.5px; }
  .link { padding: 0; border: 0; background: none; color: var(--brand-b); font: inherit; font-weight: 700; cursor: pointer; text-decoration: underline; text-underline-offset: 3px; }
</style>
