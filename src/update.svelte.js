import { check } from '@tauri-apps/plugin-updater';
import { win } from './api.js';

// Shared by the update notice and the About panel. Releases are read from GitHub (latest.json next to each release).
export const up = $state({ update: null, dismissed: false, phase: 'idle', pct: 0, err: '', error: '', checking: false, result: '' });
let last = 0; // result: '' | 'latest' | 'failed'

export async function checkNow() {
  if (!win || up.checking || up.phase !== 'idle') return;
  up.checking = true;
  up.result = '';
  up.error = '';
  last = Date.now();
  try {
    up.update = await check();
    up.dismissed = false;
    up.result = up.update ? '' : 'latest';
  } catch (e) {
    up.result = 'failed'; // offline or no release yet: the notice stays quiet, only the About button reports it
    up.error = String(e).slice(0, 160);
  }
  up.checking = false;
}

export async function install() {
  up.phase = 'downloading';
  let total = 0;
  let done = 0;
  try {
    await up.update.downloadAndInstall((e) => {
      if (e.event === 'Started') total = e.data.contentLength ?? 0;
      else if (e.event === 'Progress') {
        done += e.data.chunkLength;
        up.pct = total ? Math.min(100, Math.round((done / total) * 100)) : 0;
      } else if (e.event === 'Finished') up.phase = 'installing';
    });
    up.phase = 'installing'; // the installer takes over, then reopens the app
  } catch (e) {
    up.err = String(e);
    up.phase = 'error';
  }
}

if (win) {
  setTimeout(async () => {
    await checkNow(); // a few seconds after launch so startup stays instant
    if (up.result === 'failed') setTimeout(checkNow, 30000); // the network may not be up yet right after login
  }, 4000);
  setInterval(checkNow, 3600 * 1000); // hourly while the app stays open
  addEventListener('focus', () => Date.now() - last > 600000 && checkNow()); // and when you come back to it after 10+ minutes
}
