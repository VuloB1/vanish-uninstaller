import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

// Outside Tauri (plain `vite` in a browser) fall back to fake data so the UI can be styled.
const mock = {
  list: async () => [
    { id: 'a', name: '7-Zip 23.01 (x64)', publisher: 'Igor Pavlov', version: '23.01', installLocation: 'C:\\Program Files\\7-Zip', uninstall: 'x', sizeKb: 5632, installDate: '20240312' },
    { id: 'b', name: 'Discord', publisher: 'Discord Inc.', version: '1.0.9', installLocation: '', uninstall: 'x', sizeKb: 412000, installDate: '20250101' },
    { id: 'g', name: 'Counter-Strike 2', publisher: 'Valve', version: '1.0', installLocation: 'D:\\SteamLibrary\\steamapps\\common\\cs2', uninstall: 'x', sizeKb: 36000000, installDate: '20261020', displayIcon: '', family: '' },
    { id: 'v', name: 'Microsoft Visual C++ 2015-2022 Redistributable (x64)', publisher: 'Microsoft Corporation', version: '14.38', installLocation: '', uninstall: 'x', sizeKb: 20000, installDate: '20240101', displayIcon: '', family: '' },
    { id: 'm', name: 'Microsoft Edge', publisher: 'Microsoft Corporation', version: '120', installLocation: '', uninstall: 'x', sizeKb: 600000, installDate: '20250101', displayIcon: '', family: '' },
    { id: 'c', name: 'Notepad++', publisher: 'Notepad++ Team', version: '8.6', installLocation: '', uninstall: '', sizeKb: 14000, installDate: '' },
  ],
  admin: async () => false,
  scanJunk: async () => [
    { id: 'temp', name: 'Archivos temporales', desc: 'Carpeta Temp del usuario y de Windows', size: 504076147, files: 1309 },
    { id: 'browsers', name: 'Caché de navegadores', desc: 'Chrome, Edge, Brave y Firefox', size: 0, files: 0 },
    { id: 'gpu', name: 'Caché de shaders de la GPU', desc: 'NVIDIA, AMD y DirectX', size: 5891430310, files: 1097 },
  ],
  cleanJunk: async () => ({ freed: 504076147, skipped: 3 }),
  scanRegistry: async () => [
    { id: 'r0', program: 'Inicio automático roto', kind: 'value', path: 'HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run → Foo', confidence: 'high', reason: 'Apunta a un archivo que no existe: C:\\Foo\\foo.exe', size: 0 },
    ...Array.from({ length: 8 }, (_, i) => ({ id: `rc${i}`, program: 'Componentes COM rotos (revisar)', kind: 'reg', path: `HKLM\\SOFTWARE\\Classes\\CLSID\\{0000000${i}-0000}`, confidence: 'medium', reason: 'Falta C:\\Program Files (x86)\\Microsoft\\EdgeUpdate\\1.3.2.1\\psmachine.dll', size: 0 })),
    { id: 'r1', program: 'Rutas de aplicación rotas', kind: 'reg', path: 'HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\bar.exe', confidence: 'high', reason: 'bar.exe — falta C:\\Bar\\bar.exe', size: 0 },
  ],
  store: async () => [
    { id: 'APPX\\Demo_1.0_x64__x', name: 'Calculadora', publisher: 'Microsoft Corporation', version: '11.2', installLocation: '', uninstall: 'x', sizeKb: 0, installDate: '', displayIcon: '', family: 'Demo_x' },
  ],
  icons: async () => ({}),
  uninstall: async () => 0,
  kill: async () => {},
  openFolder: async () => {},
  scan: async () => [
    { id: 'l0', program: '7-Zip', kind: 'folder', path: 'C:\\Program Files\\7-Zip', confidence: 'high', reason: 'Carpeta de instalación', size: 5000000 },
    { id: 'l1', kind: 'reg', path: 'HKCU\\Software\\7-Zip', confidence: 'high', reason: 'Nombre coincide exactamente', size: 0 },
    { id: 'l2', kind: 'folder', path: 'C:\\Users\\x\\AppData\\Roaming\\7-Zip Helper', confidence: 'medium', reason: 'Nombre parecido', size: 12000 },
    { id: 'l3', kind: 'folder', path: 'C:\\Users\\x\\AppData\\Local\\Igor', confidence: 'low', reason: 'Carpeta del editor', size: 800 },
  ],
  remove: async (ids) => ({ moved: ids.length, gone: 1, failed: [], quarantine: 'C:\\Users\\x\\AppData\\Local\\Vanish\\quarantine' }),
};

const real = {
  list: () => invoke('list_programs'),
  admin: () => invoke('is_admin'),
  scanJunk: () => invoke('scan_junk'),
  cleanJunk: (ids) => invoke('clean_junk', { ids }),
  scanRegistry: () => invoke('scan_registry'),
  store: () => invoke('store_programs'),
  icons: () => invoke('icons'),
  uninstall: (id) => invoke('uninstall', { id }),
  kill: (id) => invoke('kill_processes', { id }),
  openFolder: (id) => invoke('open_folder', { id }),
  scan: (programs) => invoke('scan', { programs }),
  remove: (ids) => invoke('delete_leftovers', { ids }),
};

const tauri = '__TAURI_INTERNALS__' in window;
export const api = tauri ? real : mock;
export const win = tauri ? getCurrentWindow() : null;
