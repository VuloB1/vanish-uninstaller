<script>
  import { fly } from 'svelte/transition';

  // options: [[value, label], ...]. A native <select> pops up an OS-styled list, so this draws its own menu.
  let { value = $bindable(), options, label = '' } = $props();

  let open = $state(false);
  let root;
  const current = $derived(options.find(([v]) => v === value)?.[1] ?? '');

  const pick = (v) => {
    value = v;
    open = false;
  };
</script>

<svelte:window
  onclick={(e) => open && root && !root.contains(e.target) && (open = false)}
  onkeydown={(e) => e.key === 'Escape' && (open = false)}
/>

<div class="sel" bind:this={root}>
  <button class="trigger" class:open title={label} aria-label={label} aria-haspopup="listbox" aria-expanded={open} onclick={() => (open = !open)}>
    {current}
    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6" /></svg>
  </button>
  {#if open}
    <ul class="menu" role="listbox" transition:fly={{ y: -6, duration: 150 }}>
      {#each options as [v, l]}
        <li role="presentation">
          <button role="option" aria-selected={v === value} class:on={v === value} onclick={() => pick(v)}>
            {l}
            {#if v === value}<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5 9-10" /></svg>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .sel { position: relative; z-index: 4; }
  .trigger { display: flex; align-items: center; gap: 8px; padding: 8px 14px; border-radius: 999px; border: 1px solid var(--border); background: var(--card); color: var(--text); font: inherit; font-size: 13px; transition: border-color 0.18s, box-shadow 0.18s; }
  .trigger:hover, .trigger.open { border-color: var(--pill-border); }
  .trigger.open { box-shadow: 0 0 0 3px rgba(13, 148, 136, 0.12); }
  .trigger svg { color: var(--text-2); transition: transform 0.2s; }
  .trigger.open svg { transform: rotate(180deg); }

  .menu { position: absolute; right: 0; top: calc(100% + 6px); min-width: max(100%, 150px); margin: 0; padding: 4px; list-style: none; background: var(--card); border: 1px solid var(--border); border-radius: 12px; box-shadow: 0 14px 30px -12px var(--shadow); }
  .menu button { display: flex; align-items: center; justify-content: space-between; gap: 12px; width: 100%; padding: 7px 12px; border: 0; border-radius: 8px; background: transparent; color: var(--text); font: inherit; font-size: 13px; text-align: left; transition: background 0.15s; }
  .menu button:hover { background: var(--grad-soft); }
  .menu button.on { font-weight: 700; }
  .menu button.on svg { color: var(--brand-b); }
</style>
