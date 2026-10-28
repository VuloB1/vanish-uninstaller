<script>
  import { fade, fly, slide } from 'svelte/transition';
  import { api } from './api.js';
  import { kinds } from './util.js';

  let { active } = $props();

  let found = $state([]);
  let checked = $state({});
  let scanning = $state(false);
  let phase = $state('list'); // list | confirm | busy | done
  let report = $state(null);
  let error = $state('');
  let started = false;

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
      found = await api.scanRegistry();
      checked = Object.fromEntries(found.map((f) => [f.id, f.confidence === 'high'])); // "revisar" groups start unticked
    } catch (e) {
      error = String(e);
    }
    scanning = false;
  }

  const groups = $derived(
    Object.entries(
      found.reduce((g, f) => {
        (g[f.program] ??= []).push(f);
        return g;
      }, {}),
    ),
  );
  const n = $derived(found.filter((f) => checked[f.id]).length);

  // Long or "revisar" groups start collapsed so the page stays short; the header toggles them.
  let open = $state({});
  const isOpen = (name, items) => open[name] ?? (items.length <= 6 && items[0].confidence === 'high');

  const allOn = (items) => items.every((f) => checked[f.id]);
  const toggle = (items) => {
    const to = !allOn(items);
    for (const f of items) checked[f.id] = to;
  };

  async function repair() {
    phase = 'busy';
    try {
      report = await api.remove(found.filter((f) => checked[f.id]).map((f) => f.id));
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
    <span class="pill">{scanning ? 'Analizando…' : `${found.length} ${found.length === 1 ? 'problema' : 'problemas'}`}</span>
  </div>
  <button class="btn" disabled={scanning} onclick={scan}>Analizar de nuevo</button>
  <button class="btn primary" disabled={scanning || !n} onclick={() => (phase = 'confirm')}>Reparar {n}</button>
</header>

{#if error}<div class="err">{error}</div>{/if}

{#if scanning && !found.length}
  <section class="card"><div class="busy"><div class="spin"></div>Revisando el registro…</div></section>
{:else if !found.length}
  <section class="card"><div class="empty">No se encontraron problemas en el registro 🎉</div></section>
{/if}

{#each groups as [name, items], i (name)}
  <section class="card group" style="--i:{i}">
    <div class="ghead row">
      <label class="pickzone" onclick={(e) => e.stopPropagation()}>
        <input type="checkbox" class="pick" aria-label="Seleccionar todo: {name}" checked={allOn(items)} onchange={() => toggle(items)} />
      </label>
      <button class="meta gname" onclick={() => (open[name] = !isOpen(name, items))} aria-expanded={isOpen(name, items)}>
        <span class="chev" class:open={isOpen(name, items)}>▸</span>{name}
      </button>
      <span class="pill">{items.filter((f) => checked[f.id]).length}/{items.length}</span>
    </div>
    {#if isOpen(name, items)}
      <div transition:slide={{ duration: 200 }}>
        {#each items as f (f.id)}
          <label class="item">
            <input type="checkbox" bind:checked={checked[f.id]} />
            <span class="kind">{kinds[f.kind] ?? f.kind}</span>
            <span class="path">{f.path}<small>{f.reason}</small></span>
          </label>
        {/each}
      </div>
    {/if}
  </section>
{/each}

{#if phase !== 'list'}
  <div class="overlay" transition:fade={{ duration: 160 }}>
    <div class="card modal" in:fly={{ y: 14, duration: 240 }}>
      {#key phase}<div class="mc" in:fade={{ duration: 180 }}>
      {#if phase === 'confirm'}
        <h2>¿Reparar {n} {n === 1 ? 'problema' : 'problemas'}?</h2>
        <p>Cada clave se exporta a un archivo <b>.reg</b> antes de eliminarse, así que se puede restaurar con doble clic.</p>
        <p class="sub">Solo se tocan entradas que apuntan a archivos que ya no existen. Aun así, revisa la lista si algo te resulta raro.</p>
        <footer>
          <button class="btn" onclick={() => (phase = 'list')}>Cancelar</button>
          <button class="btn primary" onclick={repair}>Reparar</button>
        </footer>
      {:else if phase === 'busy'}
        <div class="busy"><div class="spin"></div>Reparando…</div>
      {:else}
        <h2>Listo</h2>
        <p>{report.moved} entradas eliminadas{report.gone ? `, ${report.gone} ya no existían` : ''}.</p>
        {#if report.moved}<p class="sub">Copias de seguridad en:</p><code>{report.quarantine}\backups</code>{/if}
        {#if report.failed.length}
          <h3>No se pudieron eliminar</h3>
          {#each report.failed as f}<div class="sub">{f}</div>{/each}
        {/if}
        <footer><button class="btn primary" onclick={() => (phase = 'list')}>Cerrar</button></footer>
      {/if}
      </div>{/key}
    </div>
  </div>
{/if}

<style>
  .group { margin-bottom: 12px; overflow: hidden; }
  .ghead { background: var(--content); }
  .gname { display: flex; align-items: center; gap: 6px; border: 0; background: none; color: var(--text); font: inherit; font-weight: 700; text-align: left; padding: 6px 0; cursor: pointer; }
  .chev { display: inline-block; font-size: 10px; color: var(--text-2); transition: transform 0.15s; }
  .chev.open { transform: rotate(90deg); }
  .group .item { padding: 8px 18px; }
  .group .item:last-child { border-bottom: 0; }
</style>
