<script>
  // Saturation/value square + hue bar + hex field, drawn with the app's own styling.
  let { value, oninput } = $props();

  const clamp = (n) => Math.min(1, Math.max(0, n));

  function toHsv(hex) {
    const n = parseInt(hex.slice(1), 16);
    const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((c) => c / 255);
    const max = Math.max(r, g, b);
    const d = max - Math.min(r, g, b);
    let h = 0;
    if (d) h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
    return [(h * 60 + 360) % 360, max ? d / max : 0, max];
  }

  function toHex(h, s, v) {
    const f = (n) => {
      const k = (n + h / 60) % 6;
      return Math.round((v - v * s * Math.max(0, Math.min(k, 4 - k, 1))) * 255);
    };
    return '#' + [f(5), f(3), f(1)].map((c) => c.toString(16).padStart(2, '0')).join('');
  }

  const init = toHsv(value);
  let h = $state(init[0]);
  let s = $state(init[1]);
  let v = $state(init[2]);
  let text = $state(value);

  // Follow outside changes (preset click, reset) without fighting the drag.
  $effect(() => {
    if (value.toLowerCase() !== toHex(h, s, v)) {
      const t = toHsv(value);
      h = t[0];
      s = t[1];
      v = t[2];
    }
    text = value;
  });

  const emit = () => oninput(toHex(h, s, v));

  function drag(e, set) {
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    const move = (ev) => {
      const r = el.getBoundingClientRect();
      set(clamp((ev.clientX - r.left) / r.width), clamp((ev.clientY - r.top) / r.height));
      emit();
    };
    move(e);
    el.onpointermove = move;
    el.onpointerup = () => (el.onpointermove = null);
  }

  function typed(e) {
    const t = e.currentTarget.value.trim();
    text = t;
    if (/^#[0-9a-f]{6}$/i.test(t)) oninput(t.toLowerCase());
  }
</script>

<div class="cp">
  <div class="sv" style="--h:{h}" onpointerdown={(e) => drag(e, (x, y) => ((s = x), (v = 1 - y)))} role="slider" aria-label="Saturation and brightness" aria-valuenow={Math.round(s * 100)} tabindex="-1">
    <i style="left:{s * 100}%; top:{(1 - v) * 100}%; background:{toHex(h, s, v)}"></i>
  </div>
  <div class="hue" onpointerdown={(e) => drag(e, (x) => (h = x * 360))} role="slider" aria-label="Hue" aria-valuenow={Math.round(h)} tabindex="-1">
    <i style="left:{(h / 360) * 100}%; background:hsl({h} 100% 50%)"></i>
  </div>
  <div class="row2">
    <span class="chip" style="background:{toHex(h, s, v)}"></span>
    <input class="hex" value={text} oninput={typed} maxlength="7" spellcheck="false" aria-label="Hex" />
  </div>
</div>

<style>
  .cp { display: grid; gap: 12px; width: 100%; padding: 12px; border: 1px solid var(--border); border-radius: 12px; background: var(--content); }
  .sv { position: relative; height: 120px; border-radius: 10px; cursor: crosshair; touch-action: none; background: linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl(var(--h) 100% 50%)); }
  .hue { position: relative; height: 14px; border-radius: 999px; cursor: ew-resize; touch-action: none; background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00); }
  i { position: absolute; width: 18px; height: 18px; transform: translate(-50%, -50%); border-radius: 50%; border: 3px solid #fff; box-shadow: 0 1px 6px rgba(0, 0, 0, 0.45); pointer-events: none; }
  .hue i { top: 50%; }
  .row2 { display: flex; align-items: center; gap: 10px; }
  .chip { width: 28px; height: 28px; flex: none; border-radius: 8px; border: 1px solid var(--border); }
  .hex { flex: 1; min-width: 0; padding: 7px 12px; border-radius: 999px; border: 1px solid var(--border); background: var(--card); color: var(--text); font: 13px Consolas, monospace; outline: none; text-transform: lowercase; }
  .hex:focus { border-color: var(--brand-b); box-shadow: 0 0 0 3px color-mix(in srgb, var(--brand-b) 15%, transparent); }
</style>
