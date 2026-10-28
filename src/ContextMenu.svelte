<script>
  import { scale } from 'svelte/transition';

  // items: [{ label, onclick, disabled?, danger?, hint? }] or the string 'sep'.
  let { x, y, items, onclose } = $props();
  let w = $state(0);
  let h = $state(0);
  // Keep the whole menu inside the window, flipping to the other side of the cursor when it would overflow.
  const left = $derived(x + w > innerWidth - 8 ? Math.max(8, x - w) : x);
  const top = $derived(y + h > innerHeight - 8 ? Math.max(8, y - h) : y);

  function pick(it) {
    if (it.disabled) return;
    onclose();
    it.onclick();
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} onblur={onclose} onresize={onclose} onwheel={onclose} />
<div class="shield" onmousedown={onclose} oncontextmenu={(e) => (e.preventDefault(), onclose())} role="presentation"></div>
<div class="cm card" role="menu" style="left:{left}px;top:{top}px" bind:clientWidth={w} bind:clientHeight={h} transition:scale={{ start: 0.96, duration: 120 }}>
  {#each items as it}
    {#if it === 'sep'}
      <div class="sep"></div>
    {:else}
      <button role="menuitem" class:danger={it.danger} disabled={it.disabled} onclick={() => pick(it)}>{it.label}</button>
    {/if}
  {/each}
</div>

<style>
  .shield { position: fixed; inset: 0; z-index: 19; }
  .cm { position: fixed; z-index: 20; min-width: 220px; padding: 5px; transform-origin: top left; box-shadow: 0 14px 34px -10px var(--shadow); }
  button { display: block; width: 100%; padding: 8px 12px; border: 0; border-radius: 8px; background: transparent; color: var(--text); font: inherit; font-size: 13.5px; font-weight: 600; text-align: left; transition: background 0.12s; }
  button:hover:not(:disabled) { background: var(--grad-soft); }
  button.danger { color: var(--danger); }
  button:disabled { opacity: 0.4; cursor: default; }
  .sep { height: 1px; margin: 4px 6px; background: var(--border); }
</style>
