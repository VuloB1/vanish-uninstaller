<script>
  import { t } from './i18n.svelte.js';

  // `p` is the latest { done, total, id, label } from the backend, or null while nothing has been reported yet.
  let { title, p = null } = $props();

  const pct = $derived(p && p.total ? Math.min(100, Math.round((p.done / p.total) * 100)) : 0);
  const item = $derived(p ? (p.id ? `${t(`junk.${p.id}.n`)}${p.label ? ' \u00b7 ' + p.label : ''}` : p.label) : '');
</script>

<div class="prog" role="status">
  <div class="top">
    <div class="spin"></div>
    <span class="title">{title}</span>
    {#if p?.total}<b>{pct}%</b>{/if}
  </div>
  {#if p?.total}
    <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct}><i style="width:{pct}%"></i></div>
    <div class="line"><span class="item" title={item}>{item}</span><span class="count">{p.done.toLocaleString()} / {p.total.toLocaleString()}</span></div>
  {/if}
</div>

<style>
  .prog { display: grid; gap: 12px; width: min(480px, 76vw); padding: 24px; }
  .top { display: flex; align-items: center; gap: 14px; font-weight: 600; }
  .title { flex: 1; min-width: 0; }
  .top b { font-variant-numeric: tabular-nums; }
  .bar { height: 8px; overflow: hidden; border-radius: 999px; background: color-mix(in srgb, var(--brand-b) 14%, var(--content)); }
  .bar i { display: block; height: 100%; border-radius: inherit; background: var(--grad); transition: width 0.15s linear; }
  .line { display: flex; gap: 14px; justify-content: space-between; font-size: 12px; color: var(--text-2); }
  .item { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: Consolas, monospace; }
  .count { flex: none; font-variant-numeric: tabular-nums; }
</style>
