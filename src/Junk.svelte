<script>
  import { fade, fly } from 'svelte/transition';
  import { api } from './api.js';
  import { fmt } from './util.js';
  import { t, tb, locale } from './i18n.svelte.js';

  let { active } = $props();

  let cats = $state([]);
  let checked = $state({});
  let scanning = $state(false);
  let phase = $state('list'); // list | confirm | busy | done
  let report = $state(null);
  let error = $state('');
  let started = false;

  // Scans the first time the tab is opened (it can take several seconds on a big disk).
  $effect(() => {
    if (active && !started) {
      started = true;
      scan();
    }
  });

  async function scan() {
    scanning = true;
    error = '';
    try {
      cats = await api.scanJunk();
      checked = Object.fromEntries(cats.map((c) => [c.id, c.size > 0]));
    } catch (e) {
      error = tb(String(e));
    }
    scanning = false;
  }

  const ids = $derived(cats.filter((c) => checked[c.id] && c.size > 0).map((c) => c.id));
  const selBytes = $derived(cats.filter((c) => checked[c.id]).reduce((a, c) => a + c.size, 0));
  const totalBytes = $derived(cats.reduce((a, c) => a + c.size, 0));

  async function clean() {
    phase = 'busy';
    try {
      report = await api.cleanJunk(ids);
      phase = 'done';
    } catch (e) {
      error = tb(String(e));
      phase = 'list';
    }
    scan();
  }
</script>

<header class="bar">
  <div class="pills">
    {#if scanning}
      <span class="pill">{t('btn.scanning')}</span>
    {:else}
      <span class="pill">{t('j.pill', { size: fmt(totalBytes) || '0 B' })}</span>
    {/if}
  </div>
  <button class="btn" disabled={scanning} onclick={scan}>{t('btn.rescan')}</button>
  <button class="btn primary" disabled={scanning || !ids.length} onclick={() => (phase = 'confirm')}>{t('j.clean', { size: fmt(selBytes) })}</button>
</header>

{#if error}<div class="err">{error}</div>{/if}

<section class="card">
  {#if scanning && !cats.length}
    <div class="busy"><div class="spin"></div>{t('j.searching')}</div>
  {/if}
  {#each cats as c, i (c.id)}
    <div class="row" style="--i:{i}">
      <label class="pickzone" class:off={!c.size} onclick={(e) => e.stopPropagation()}>
        <input type="checkbox" class="pick" aria-label={t('p.pickName', { name: t(`junk.${c.id}.n`) })} disabled={!c.size} bind:checked={checked[c.id]} />
      </label>
      <div class="meta">
        <div class="name">{t(`junk.${c.id}.n`)}</div>
        <div class="sub">{t(`junk.${c.id}.d`)}</div>
      </div>
      <div class="num">
        <div class="amount">{c.size ? fmt(c.size) : t('j.clear')}</div>
        {#if c.files}<div class="sub">{t('j.files', { n: c.files.toLocaleString(locale()) })}</div>{/if}
      </div>
    </div>
  {/each}
</section>

{#if phase !== 'list'}
  <div class="overlay" transition:fade={{ duration: 160 }}>
    <div class="card modal" in:fly={{ y: 14, duration: 240 }}>
      {#key phase}<div class="mc" in:fade={{ duration: 180 }}>
      {#if phase === 'confirm'}
        <h2>{t('j.confirm.title', { size: fmt(selBytes) })}</h2>
        <p>{@html t('j.confirm.p1')}</p>
        <p class="sub">{t('j.confirm.p2')}</p>
        <footer>
          <button class="btn" onclick={() => (phase = 'list')}>{t('btn.cancel')}</button>
          <button class="btn primary" onclick={clean}>{t('btn.clear')}</button>
        </footer>
      {:else if phase === 'busy'}
        <div class="busy"><div class="spin"></div>{t('j.busy')}</div>
      {:else}
        <h2>{t('done.title')}</h2>
        <p>{@html t('j.done', { size: fmt(report.freed) || '0 B' })}</p>
        {#if report.skipped}<p class="sub">{t('j.skipped', { n: report.skipped.toLocaleString(locale()) })}</p>{/if}
        <footer><button class="btn primary" onclick={() => (phase = 'list')}>{t('btn.close')}</button></footer>
      {/if}
      </div>{/key}
    </div>
  </div>
{/if}

<style>
  .num { text-align: right; }
  .amount { font-weight: 700; white-space: nowrap; }
</style>
