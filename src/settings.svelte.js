// Accent color: a preset (gradient pair) or one custom color whose partner is the same color with its hue shifted.
export const PRESETS = [
  ['teal', '#0d9488', '#f472a8'],
  ['blue', '#2563eb', '#22d3ee'],
  ['violet', '#7c3aed', '#f25f7d'],
  ['orange', '#ea580c', '#f43f5e'],
  ['green', '#16a34a', '#84cc16'],
  ['graphite', '#475569', '#94a3b8'],
];

const load = () => {
  try {
    const a = JSON.parse(localStorage.getItem('accent'));
    if (a?.id === 'custom' && /^#[0-9a-f]{6}$/i.test(a.b)) return a;
    if (PRESETS.some(([id]) => id === a?.id)) return { id: a.id };
  } catch {}
  return { id: 'teal' };
};

let accent = $state(load());

export const getAccent = () => accent;

/** [main, partner] CSS colors for an accent. */
export function colors(a) {
  if (a.id === 'custom') return [a.b, `oklch(from ${a.b} l c calc(h + 40))`];
  const [, b, p] = PRESETS.find(([id]) => id === a.id);
  return [b, p];
}

function apply() {
  const [b, a] = colors(accent);
  document.documentElement.style.setProperty('--brand-b', b);
  document.documentElement.style.setProperty('--brand-a', a);
}
apply(); // before first paint, so the default colors never flash

// Background level, -100 (darker) .. 100 (lighter). The stylesheet turns --drop / --lift into surface colors for both themes.
const loadLevel = () => {
  try {
    const n = Number(localStorage.getItem('level'));
    return Number.isFinite(n) ? Math.max(-100, Math.min(100, n)) : 0;
  } catch {
    return 0;
  }
};
let level = $state(loadLevel());
export const getLevel = () => level;

function applyLevel() {
  document.documentElement.style.setProperty('--drop', Math.max(-level, 0));
  document.documentElement.style.setProperty('--lift', Math.max(level, 0));
}
applyLevel();

export function setLevel(n) {
  level = n;
  applyLevel();
  try { localStorage.setItem('level', String(n)); } catch {}
}

export function setAccent(a) {
  accent = a;
  apply();
  try { localStorage.setItem('accent', JSON.stringify(a)); } catch {}
}
