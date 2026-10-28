// Builds latest.json for the Tauri updater from the signed NSIS build.
// Usage: node scripts/latest-json.mjs "release notes"   (after `npm run tauri build` with the signing key set)
import { readFileSync, writeFileSync } from 'node:fs';

const { version } = JSON.parse(readFileSync('package.json', 'utf8'));
const dir = 'src-tauri/target/release/bundle/nsis';
const signature = readFileSync(`${dir}/Vanish Uninstaller_${version}_x64-setup.exe.sig`, 'utf8').trim();
const asset = `Vanish-Uninstaller_${version}_setup.exe`; // the name the installer is uploaded under
const url = `https://github.com/VuloB1/vanish-uninstaller/releases/download/v${version}/${asset}`;

const json = { version, notes: process.argv[2] ?? '', pub_date: new Date().toISOString(), platforms: { 'windows-x86_64': { signature, url } } };
writeFileSync('release/latest.json', JSON.stringify(json, null, 2));
console.log(`release/latest.json for v${version}`);
