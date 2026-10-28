<script>
  import { fade, fly } from 'svelte/transition';
  import { api } from './api.js';
  import { fmt } from './util.js';

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
      error = String(e);
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
      error = String(e);
      phase = 'list';
    }
    scan();
  }
</script>

<header class="bar">
  <div class="pills">
    {#if scanning}
      <span class="pill">Analizando…</span>
    {:else}
      <span class="pill">{fmt(totalBytes) || '0 B'} de archivos innecesarios</span>
    {/if}
  </div>
  <button class="btn" disabled={scanning} onclick={scan}>Analizar de nuevo</button>
  <button class="btn primary" disabled={scanning || !ids.length} onclick={() => (phase = 'confirm')}>Limpiar {fmt(selBytes)}</button>
</header>

{#if error}<div class="err">{error}</div>{/if}

<section class="card">
  {#if scanning && !cats.length}
    <div class="busy"><div class="spin"></div>Buscando archivos innecesarios…</div>
  {/if}
  {#each cats as c, i (c.id)}
    <div class="row" style="--i:{i}">
      <label class="pickzone" class:off={!c.size} onclick={(e) => e.stopPropagation()}>
        <input type="checkbox" class="pick" aria-label="Seleccionar {c.name}" disabled={!c.size} bind:checked={checked[c.id]} />
      </label>
      <div class="meta">
        <div class="name">{c.name}</div>
        <div class="sub">{c.desc}</div>
      </div>
      <div class="num">
        <div class="amount">{c.size ? fmt(c.size) : 'Limpio'}</div>
        {#if c.files}<div class="sub">{c.files.toLocaleString('es')} archivos</div>{/if}
      </div>
    </div>
  {/each}
</section>

{#if phase !== 'list'}
  <div class="overlay" transition:fade={{ duration: 160 }}>
    <div class="card modal" in:fly={{ y: 14, duration: 240 }}>
      {#key phase}<div class="mc" in:fade={{ duration: 180 }}>
      {#if phase === 'confirm'}
        <h2>¿Limpiar {fmt(selBytes)}?</h2>
        <p>Son cachés y temporales que las aplicaciones vuelven a generar. Se eliminan <b>de forma permanente</b>: no pasan por la cuarentena, porque no tendría sentido guardar gigas de caché.</p>
        <p class="sub">Los archivos que estén en uso se omiten. Para liberar más espacio, cierra antes los navegadores y los juegos.</p>
        <footer>
          <button class="btn" onclick={() => (phase = 'list')}>Cancelar</button>
          <button class="btn primary" onclick={clean}>Limpiar</button>
        </footer>
      {:else if phase === 'busy'}
        <div class="busy"><div class="spin"></div>Limpiando…</div>
      {:else}
        <h2>Listo</h2>
        <p>Se han liberado <b>{fmt(report.freed) || '0 B'}</b>.</p>
        {#if report.skipped}<p class="sub">{report.skipped.toLocaleString('es')} elementos estaban en uso o protegidos y se han dejado.</p>{/if}
        <footer><button class="btn primary" onclick={() => (phase = 'list')}>Cerrar</button></footer>
      {/if}
      </div>{/key}
    </div>
  </div>
{/if}

<style>
  .num { text-align: right; }
  .amount { font-weight: 700; white-space: nowrap; }
</style>
