<script>
  import { fade, fly } from 'svelte/transition';
  import { api } from './api.js';
  import Select from './Select.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import { fmt, date, kinds } from './util.js';

  let programs = $state([]);
  let icons = $state({});
  let picked = $state({});
  let notice = $state('');
  let query = $state('');
  let cat = $state('all');
  let sort = $state('name');
  let sel = $state(null);
  let phase = $state('list'); // list | busy | force | confirm | results | done
  let msg = $state('');
  let error = $state('');
  let found = $state([]);
  let checked = $state({});
  let report = $state(null);
  let cur = $state(null);
  let menu = $state(null); // { x, y, p } while the right-click menu is open

  // Only offers what the program can actually do: no folder entries for Store apps, no uninstall without an uninstaller.
  function menuItems(p) {
    const n = batch.length;
    const on = !!picked[p.id];
    const folder = p.installLocation && !p.family;
    return [
      { label: 'Desinstalar', disabled: !p.uninstall, onclick: () => run([p], true) },
      { label: 'Buscar restos', onclick: () => run([p], false) },
      ...(!p.family ? [{ label: 'Forzar eliminación…', danger: true, onclick: () => ((cur = [p]), (phase = 'force')) }] : []),
      'sep',
      { label: on ? 'Quitar de la selección' : 'Añadir a la selección', disabled: !p.uninstall, onclick: () => (picked[p.id] = !on) },
      ...(n ? [{ label: `Desinstalar los ${n} seleccionados`, onclick: () => (phase = 'confirm') }, { label: 'Limpiar selección', onclick: () => (picked = {}) }] : []),
      'sep',
      ...(folder ? [{ label: 'Abrir carpeta de instalación', onclick: () => api.openFolder(p.id).catch((e) => (error = String(e))) }] : []),
      { label: 'Copiar nombre', onclick: () => navigator.clipboard?.writeText(p.name) },
      ...(folder ? [{ label: 'Copiar ruta de instalación', onclick: () => navigator.clipboard?.writeText(p.installLocation) }] : []),
    ];
  }

  // Categories are guessed from name, publisher and install folder; "recent" and "big" are filters, not categories.
  const CATS = [
    ['all', 'Todos'],
    ['apps', 'Aplicaciones'],
    ['games', 'Juegos'],
    ['system', 'Drivers y runtimes'],
    ['microsoft', 'Microsoft'],
    ['store', 'Tienda'],
    ['recent', 'Recientes'],
    ['big', 'Grandes'],
  ];
  const ICONS = {
    all: 'M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z',
    apps: 'M5 4h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2zM3 9h18',
    games: 'M6 12h4M8 10v4M15 11h.01M18 13h.01M17.3 5H6.7a4 4 0 0 0-3.9 3.1l-1.3 6a3.2 3.2 0 0 0 5.6 2.6L8.5 15h7l1.4 1.7a3.2 3.2 0 0 0 5.6-2.6l-1.3-6A4 4 0 0 0 17.3 5z',
    system: 'M7 5h10a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2zM9 9h6v6H9zM9 2v3M15 2v3M9 19v3M15 19v3M2 9h3M2 15h3M19 9h3M19 15h3',
    microsoft: 'M3 3h8.5v8.5H3zM12.5 3H21v8.5h-8.5zM3 12.5h8.5V21H3zM12.5 12.5H21V21h-8.5z', // Windows 11 logo (filled)
    store: 'M6 2L3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4zM3 6h18M16 10a4 4 0 0 1-8 0',
    recent: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 7v5l3 2',
    big: 'M22 12H2M5.5 5.1L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.5-6.9A2 2 0 0 0 16.7 4H7.3a2 2 0 0 0-1.8 1.1zM6 16h.01M10 16h.01',
  };
  const SYS =/visual c\+\+|\.net|runtime|redistributable|redist|driver|directx|webview2|vulkan|physx|openal|\bjava\b|nvidia|\bamd\b|intel\(r\)|realtek|chipset|bluetooth|graphics/i;
  const GAME = /steam|epic games|ubisoft|riot|gog\.com|battle\.net|blizzard|electronic arts|rockstar|valve|\bgames?\b|juego/i;
  const catOf = (p) =>
    p.family ? 'store'
    : SYS.test(p.name) ? 'system'
    : GAME.test(`${p.name} ${p.publisher}`) || /steamapps|epic games|\\games\\/i.test(p.installLocation) ? 'games'
    : /microsoft/i.test(p.publisher) ? 'microsoft'
    : 'apps';
  const recentFrom = new Date(Date.now() - 30 * 864e5).toISOString().slice(0, 10).replaceAll('-', '');
  const inCat = (p, c) => c === 'all' || (c === 'recent' ? p.installDate >= recentFrom : c === 'big' ? p.sizeKb >= 512000 : catOf(p) === c);
  const SORTS = [['name', 'Nombre'], ['size', 'Tamaño'], ['date', 'Fecha']];
  const sorters = {
    name: (a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()),
    size: (a, b) => b.sizeKb - a.sizeKb,
    date: (a, b) => b.installDate.localeCompare(a.installDate),
  };

  const counts = $derived(Object.fromEntries(CATS.map(([id]) => [id, programs.filter((p) => inCat(p, id)).length])));
  const shown = $derived(
    programs
      .filter((p) => inCat(p, cat) && `${p.name} ${p.publisher}`.toLowerCase().includes(query.toLowerCase()))
      .sort(sorters[sort]),
  );
  const levels = [
    ['high', 'Muy probable que sea del programa'],
    ['medium', 'Posible, revisa antes de borrar'],
    ['low', 'Dudoso (puede ser compartido)'],
  ];
  const nSel = $derived(found.filter((f) => checked[f.id]).length);
  const totalKb = $derived(programs.reduce((a, p) => a + p.sizeKb, 0));
  const batch = $derived(programs.filter((p) => picked[p.id]));
  const batchKb = $derived(batch.reduce((a, p) => a + p.sizeKb, 0));
  const current = $derived(programs.find((p) => p.id === sel));

  // Registry programs show first; Store apps (slower, via PowerShell) join the list when ready.
  async function load() {
    programs = await api.list();
    api.icons().then((m) => (icons = { ...icons, ...m }));
    api.store().then((s) => {
      programs = [...programs.filter((p) => !p.family), ...s];
      icons = { ...icons, ...Object.fromEntries(s.filter((p) => p.displayIcon).map((p) => [p.id, p.displayIcon])) };
    });
  }
  load();

  // Uninstalls each program in turn (every uninstaller shows its own window), then scans leftovers for all at once.
  async function run(ps, doUninstall) {
    cur = ps;
    error = notice = '';
    try {
      phase = 'busy';
      let toScan = ps;
      if (doUninstall) {
        for (const [i, p] of ps.entries()) {
          msg = `Desinstalando ${i + 1}/${ps.length}: ${p.name}…`;
          await api.uninstall(p.id).catch((e) => (error += `${p.name}: ${e}\n`));
        }
        // An uninstaller that was cancelled leaves the program installed: its folder must not be offered for deletion.
        const [reg, store] = await Promise.all([api.list(), ps.some((p) => p.family) ? api.store() : []]);
        const now = [...reg, ...store];
        const still = ps.filter((p) => now.some((n) => n.id === p.id));
        toScan = ps.filter((p) => !still.includes(p));
        if (still.length) notice = `Siguen instalados (cancelado o aún en curso): ${still.map((p) => p.name).join(', ')}`;
      }
      picked = {};
      if (!toScan.length) {
        phase = 'list';
        load();
        return;
      }
      msg = 'Buscando restos…';
      found = await api.scan(toScan);
      cur = toScan;
      checked = Object.fromEntries(found.map((f) => [f.id, f.confidence === 'high']));
      phase = 'results';
    } catch (e) {
      error = String(e);
      phase = 'list';
    }
  }

  // Forced removal: skip the broken uninstaller, close the program's processes, then treat everything linked to it as leftovers.
  async function force(p) {
    phase = 'busy';
    msg = 'Cerrando los procesos del programa…';
    error = '';
    await api.kill(p.id).catch((e) => (error = `No se pudieron cerrar los procesos: ${e}`));
    await run([p], false);
  }

  async function purge() {
    phase = 'busy';
    msg = 'Moviendo a cuarentena…';
    try {
      report = await api.remove(found.filter((f) => checked[f.id]).map((f) => f.id));
      phase = 'done';
    } catch (e) {
      error = String(e);
      phase = 'list';
    }
    sel = null;
    load();
  }
</script>

<header class="bar">
  <div class="pills">
    <span class="pill" title="{programs.length} programas, {fmt(totalKb * 1024)} en total">{programs.length} · {fmt(totalKb * 1024)}</span>
  </div>
  <!-- Icon-only categories; the active one also shows its name and count. Hover any icon for its tooltip. -->
  <div class="seg" role="group" aria-label="Categoría">
    {#each CATS as [id, label]}
      {#if id === 'all' || counts[id]}
        <button class="segb" class:on={cat === id} title="{label} ({counts[id]})" aria-label={label} aria-pressed={cat === id} onclick={() => (cat = id)}>
          <svg class:fill={id === 'microsoft'} viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d={ICONS[id]} /></svg>
          {#if cat === id}<span>{label} <b>{counts[id]}</b></span>{/if}
        </button>
      {/if}
    {/each}
  </div>
  <Select bind:value={sort} options={SORTS} label="Ordenar por" />
  <input class="search" placeholder="Buscar…" bind:value={query} />
</header>

{#if notice}<div class="warn">{notice}</div>{/if}
{#if error}<div class="err">{error}</div>{/if}

<section class="card list">
  {#each shown as p, i (p.id)}
    <div class="row prow" style="--i:{i}" class:sel={sel === p.id} onclick={() => (sel = p.id)} oncontextmenu={(e) => (e.preventDefault(), (sel = p.id), (menu = { x: e.clientX, y: e.clientY, p }))} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && (sel = p.id)}>
      <label class="pickzone" class:off={!p.uninstall} title={p.uninstall ? 'Seleccionar' : 'Sin desinstalador'} onclick={(e) => e.stopPropagation()}>
        <input type="checkbox" class="pick" aria-label="Seleccionar {p.name}" disabled={!p.uninstall} bind:checked={picked[p.id]} />
      </label>
      {#if icons[p.id]}
        <img class="ico" src={icons[p.id]} alt="" />
      {:else}
        <div class="ico ph">{p.name.trim()[0]?.toUpperCase()}</div>
      {/if}
      <div class="meta">
        <div class="name">{p.name}{#if p.family}<span class="tag">Store</span>{/if}</div>
        <div class="sub">{[p.publisher, p.version, date(p.installDate)].filter(Boolean).join(' · ')}</div>
      </div>
      <div class="size">{fmt(p.sizeKb * 1024)}</div>
    </div>
  {:else}
    <div class="empty">Nada que mostrar</div>
  {/each}
</section>

{#if menu}<ContextMenu x={menu.x} y={menu.y} items={menuItems(menu.p)} onclose={() => (menu = null)} />{/if}

<!-- Bottom dock: sticks to the window's bottom edge while scrolling and appears below the list, so nothing above it moves. -->
{#if current || batch.length}
  <section class="card dock" class:both={current && batch.length} transition:fly={{ y: 16, duration: 220 }}>
    {#if current}
      {@const p = current}
      <div class="det">
        {#if icons[p.id]}
          <img class="ico" src={icons[p.id]} alt="" />
        {:else}
          <div class="ico ph">{p.name.trim()[0]?.toUpperCase()}</div>
        {/if}
        <div class="info">
          <div class="name">{p.name}{#if p.family}<span class="tag">Store</span>{/if}</div>
          <div class="sub">{[p.publisher, p.version && `v${p.version}`, date(p.installDate), p.sizeKb ? fmt(p.sizeKb * 1024) : ''].filter(Boolean).join(' · ')}</div>
          {#if p.installLocation}<div class="kv" title={p.installLocation}>Carpeta: <code>{p.installLocation}</code></div>{/if}
          {#if p.uninstall && !p.family}<div class="kv" title={p.uninstall}>Desinstalador: <code>{p.uninstall}</code></div>{/if}
        </div>
        <div class="acts">
          <button class="btn" onclick={() => run([p], false)}>Buscar restos</button>
          {#if !p.family}<button class="btn danger" onclick={() => ((cur = [p]), (phase = 'force'))}>Forzar</button>{/if}
          <button class="btn primary" disabled={!p.uninstall} onclick={() => run([p], true)}>Desinstalar</button>
          <button class="x" title="Quitar selección" aria-label="Quitar selección" onclick={() => (sel = null)}>×</button>
        </div>
      </div>
    {/if}
    {#if current && batch.length}<span class="vr"></span>{/if}
    {#if batch.length}
      <div class="bat">
        <div>
          <div class="name">{batch.length} {batch.length === 1 ? 'seleccionado' : 'seleccionados'}</div>
          <div class="sub">{fmt(batchKb * 1024)} en total</div>
        </div>
        <div class="acts">
          <button class="btn" onclick={() => (picked = {})}>Limpiar</button>
          <button class="btn primary" onclick={() => (phase = 'confirm')}>Desinstalar {batch.length}</button>
        </div>
      </div>
    {/if}
  </section>
{/if}

{#if phase !== 'list'}
  <div class="overlay" transition:fade={{ duration: 160 }}>
    <div class="card modal" in:fly={{ y: 14, duration: 240 }}>
      {#key phase}<div class="mc" in:fade={{ duration: 180 }}>
      {#if phase === 'busy'}
        <div class="busy"><div class="spin"></div>{msg}</div>
      {:else if phase === 'force'}
        <h2>Eliminación forzada de {cur[0].name}</h2>
        <p>No se ejecutará su desinstalador. Se cerrarán sus procesos y se buscará todo lo vinculado: carpeta de instalación, entrada de desinstalación, servicios y claves del registro.</p>
        <p class="sub">Úsalo solo si el desinstalador normal no funciona. Podrás revisar la lista antes de eliminar nada, y todo va a cuarentena.</p>
        <footer>
          <button class="btn" onclick={() => (phase = 'list')}>Cancelar</button>
          <button class="btn danger solid" onclick={() => force(cur[0])}>Cerrar procesos y buscar</button>
        </footer>
      {:else if phase === 'confirm'}
        <h2>¿Desinstalar {batch.length} programas?</h2>
        <p class="sub">Se abrirá el desinstalador de cada uno, uno tras otro. Después se buscarán sus restos.</p>
        <div class="results">
          {#each batch as p (p.id)}<div class="item"><span class="name">{p.name}</span></div>{/each}
        </div>
        <footer>
          <button class="btn" onclick={() => (phase = 'list')}>Cancelar</button>
          <button class="btn primary" onclick={() => run(batch, true)}>Desinstalar</button>
        </footer>
      {:else if phase === 'results'}
        <h2>Restos de {cur.length === 1 ? cur[0].name : `${cur.length} programas`}</h2>
        <p class="sub">Nada se borra de golpe: los archivos van a una cuarentena y las claves de registro se exportan a .reg antes de eliminarse.</p>
        <div class="results">
          {#each levels as [c, title]}
            {@const items = found.filter((f) => f.confidence === c)}
            {#if items.length}
              <h3>{title} <span class="pill">{items.length}</span></h3>
              {#each items as f (f.id)}
                <label class="item">
                  <input type="checkbox" bind:checked={checked[f.id]} />
                  <span class="kind">{kinds[f.kind] ?? 'Archivo'}</span>
                  <span class="path">{f.path}<small>{cur.length > 1 ? `${f.program} · ` : ''}{f.reason}</small></span>
                  <span class="size">{fmt(f.size)}</span>
                </label>
              {/each}
            {/if}
          {/each}
          {#if !found.length}<div class="empty">No se encontraron restos 🎉</div>{/if}
        </div>
        <footer>
          <button class="btn" onclick={() => (phase = 'list')}>Cerrar</button>
          <button class="btn primary" disabled={!nSel} onclick={purge}>Eliminar {nSel} seleccionados</button>
        </footer>
      {:else}
        <h2>Listo</h2>
        <p>
          {report.moved} elementos eliminados{report.gone ? `, ${report.gone} ya los había borrado el propio desinstalador` : ''}.
        </p>
        {#if report.moved}<p class="sub">Copia de seguridad en:</p><code>{report.quarantine}</code>{/if}
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
  .segb svg.fill { fill: currentColor; stroke: none; }
  .segb span { animation: labelIn 0.22s ease both; white-space: nowrap; }
  .acts { animation: labelIn 0.2s ease both; }
  .search { flex: 0 1 190px; min-width: 110px; width: auto; padding: 10px 16px; border-radius: 999px; border: 1px solid var(--border); background: var(--card); color: var(--text); font: inherit; outline: none; }
  .search:focus { border-color: var(--brand-b); box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.15); }

  .seg { display: flex; gap: 2px; padding: 3px; border: 1px solid var(--border); border-radius: 999px; background: var(--card); }
  .segb { display: flex; align-items: center; gap: 6px; padding: 6px 10px; border: 0; border-radius: 999px; background: transparent; color: var(--text-2); font-size: 13px; font-weight: 600; transition: background 0.15s, color 0.15s; }
  .segb:hover { color: var(--text); background: var(--grad-soft); }
  .segb.on { color: var(--text); background: var(--grad-soft); box-shadow: inset 0 0 0 1px var(--pill-border); }
  .segb b { font-weight: 600; opacity: 0.6; }

  .list { overflow: hidden; }
  .prow { cursor: pointer; outline: none; }
  .prow:hover { background: var(--content); }
  .prow.sel { background: var(--grad-soft); }
  .ico { width: 32px; height: 32px; flex: none; border-radius: 8px; object-fit: contain; }
  .ph { display: grid; place-items: center; background: var(--grad); color: #fff; font-weight: 700; }
  .tag { margin-left: 8px; padding: 1px 8px; border-radius: 999px; font-size: 10.5px; font-weight: 600; vertical-align: 1px; background: var(--grad-soft); border: 1px solid var(--pill-border); }
  .acts { display: flex; align-items: center; gap: 8px; flex: none; }

  .dock { position: sticky; bottom: 16px; z-index: 3; margin-top: 14px; display: flex; align-items: center; gap: 16px; padding: 14px 18px; box-shadow: 0 -8px 28px -12px var(--shadow), 0 6px 14px -10px var(--shadow); }
  .det { flex: 1; min-width: 0; display: flex; align-items: center; gap: 14px; }
  .dock .ico { width: 44px; height: 44px; border-radius: 10px; font-size: 18px; }
  .info { flex: 1; min-width: 0; }
  .kv { margin-top: 2px; font-size: 12px; color: var(--text-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .kv code { font-family: Consolas, monospace; }
  .vr { align-self: stretch; width: 1px; background: var(--border); }
  .bat { display: flex; align-items: center; gap: 16px; }
  .dock.both { flex-direction: column; align-items: stretch; gap: 10px; }
  .both .det { flex: none; }
  .both .vr { align-self: auto; width: auto; height: 1px; }
  .both .bat { justify-content: space-between; }
  .x { width: 28px; height: 28px; border: 0; border-radius: 50%; background: transparent; color: var(--text-2); font-size: 18px; line-height: 1; }
  .x:hover { background: var(--grad-soft); color: var(--text); }
</style>
