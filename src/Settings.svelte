<script>
  import { fade, fly, slide } from 'svelte/transition';
  import { api } from './api.js';
  import { t, LANGS, getLang, setLang } from './i18n.svelte.js';
  import { PRESETS, getAccent, setAccent, colors, getLevel, setLevel, getTheme, setTheme } from './settings.svelte.js';
  import ColorPicker from './ColorPicker.svelte';
  import Select from './Select.svelte';
  import { up, checkNow } from './update.svelte.js';
  import { win } from './api.js';
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
  let lang = $state(getLang());
  $effect(() => {
    if (lang !== getLang()) setLang(lang);
  });

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

      <h3>{t('set.theme')}</h3>
      <div class="chips" role="group" aria-label={t('set.theme')}>
        <button class="chip" class:on={getTheme() === 'light'} aria-pressed={getTheme() === 'light'} onclick={() => setTheme('light')}>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></svg>
          {t('app.themeLight')}
        </button>
        <button class="chip" class:on={getTheme() === 'dark'} aria-pressed={getTheme() === 'dark'} onclick={() => setTheme('dark')}>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" /></svg>
          {t('app.themeDark')}
        </button>
      </div>

      <h3>{t('app.lang')}</h3>
      <div class="langsel"><Select bind:value={lang} options={LANGS.map(([code, name]) => [code, name])} label={t('app.lang')} inline /></div>

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
        {#if win}
          <button class="btn chk" disabled={up.checking} onclick={checkNow}>{up.checking ? t('up.checking') : t('up.check')}</button>
        {/if}
      </div>
      {#if win}
        <!-- Always one line tall, so the button above never shifts when the result appears. -->
        <div class="upd sub" class:ok={up.update || up.result === 'latest'}>
          {#if up.update}{t('up.available', { v: up.update.version })}{:else if up.result === 'latest'}{t('up.latest')}{:else if up.result === 'failed'}{t('up.checkFailed')}{up.error ? `: ${up.error}` : ''}{/if}
        </div>
      {/if}
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
  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .chip { display: inline-flex; align-items: center; gap: 7px; padding: 7px 14px; border-radius: 999px; border: 1px solid var(--border); background: var(--card); color: var(--text); font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; transition: background 0.15s, border-color 0.15s; }
  .chip:hover { border-color: var(--pill-border); }
  .chip.on { background: var(--grad-soft); border-color: var(--pill-border); }
  .chip svg { color: var(--brand-b); }
  .picker { margin: 6px 0 4px; }
  .set > :global(*) { flex-shrink: 0; }

  .tone { display: flex; align-items: center; gap: 12px; }
  .tone .sub { white-space: nowrap; }
  .tone input { flex: 1; min-width: 0; height: 22px; margin: 0; background: transparent; appearance: none; cursor: pointer; }
  .tone input::-webkit-slider-runnable-track { height: 10px; border-radius: 999px; border: 1px solid var(--border); background: linear-gradient(to right, #050506, #3a3a3d, #77777b); }
  :global(:root[data-theme='light']) .tone input::-webkit-slider-runnable-track { background: linear-gradient(to right, #b8b8b4, #f7f7f4, #ffffff); }
  .tone input::-webkit-slider-thumb { appearance: none; width: 20px; height: 20px; margin-top: -6px; border-radius: 50%; border: 3px solid #fff; background: var(--brand-b); box-shadow: 0 1px 6px rgba(0, 0, 0, 0.4); }
  .reset { align-self: flex-start; margin-top: 10px; padding: 6px 14px; font-size: 13px; }

  .about { display: flex; align-items: center; gap: 14px; margin-bottom: 8px; }
  .chk { margin-left: auto; min-width: 170px; padding: 6px 14px; font-size: 13px; }
  .upd { min-height: 20px; text-align: right; }
  .ok { color: var(--brand-b); font-weight: 600; }
  .langsel { display: flex; }
  .about .name { font-size: 18px; font-weight: 800; letter-spacing: -0.01em; }
  .grad { background: var(--grad); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .credits { display: grid; gap: 4px; margin-top: 8px; font-size: 13.5px; }
  .link { padding: 0; border: 0; background: none; color: var(--brand-b); font: inherit; font-weight: 700; cursor: pointer; text-decoration: underline; text-underline-offset: 3px; }
</style>
