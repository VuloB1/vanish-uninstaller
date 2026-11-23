use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

/// Contents of `dir` that get cleaned (never `dir` itself). `prefix` limits it to files starting with it.
struct Target {
    dir: PathBuf,
    prefix: Option<&'static str>,
    min_age: u64, // seconds; anything modified more recently is left alone
}

struct Cat {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    targets: Vec<Target>,
}

/// Categories that start unticked: they trade a little speed or privacy history for space, so they're the user's call.
const OFF_BY_DEFAULT: [&str; 5] = ["prefetch", "fontcache", "store", "recent", "recycle"];

#[derive(Serialize)]
pub struct JunkCat {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    size: u64,
    files: u64,
    on: bool,
}

#[derive(Serialize)]
pub struct JunkReport {
    freed: u64,
    skipped: u64,
}

fn plain(dir: PathBuf) -> Target {
    Target { dir, prefix: None, min_age: 0 }
}

fn aged(dir: PathBuf) -> Target {
    Target { dir, prefix: None, min_age: 86_400 }
}

fn files(dir: PathBuf, prefix: &'static str) -> Target {
    Target { dir, prefix: Some(prefix), min_age: 0 }
}

/// Browser profile folders (`Default`, `Profile 1`, ...) under a `User Data`-style directory.
fn profiles(root: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(root) else { return vec![] };
    rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()
}

/// Where the user's own cleaner folders are kept (`VANISH_DATA` only exists so tests don't touch the real file).
fn custom_file() -> PathBuf {
    let base = std::env::var_os("VANISH_DATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var("LocalAppData").unwrap_or_default()).join("Vanish"));
    base.join("custom.json")
}

fn custom_raw() -> Vec<String> {
    fs::read_to_string(custom_file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// A folder the cleaner may be pointed at: a real directory (not a link, which would redirect the wipe elsewhere) that is
/// not something general like Windows, Program Files, a drive root or the user's Documents.
fn usable(p: &Path) -> bool {
    fs::symlink_metadata(p).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink()) && crate::leftovers::program_dir(p).is_some()
}

/// The user's folders that still exist and are still allowed. Re-checked on every use, never trusted from the file.
pub fn custom_list() -> Vec<String> {
    custom_raw().into_iter().filter(|p| usable(Path::new(p))).collect()
}

fn custom_save(list: &[String]) -> Result<(), String> {
    let f = custom_file();
    fs::create_dir_all(f.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(f, serde_json::to_string_pretty(list).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub fn custom_add(path: &str) -> Result<Vec<String>, String> {
    let p = PathBuf::from(path.trim().trim_matches('"').trim_end_matches('\\'));
    fs::symlink_metadata(&p).map_err(|_| "la ruta no existe")?;
    if !usable(&p) {
        return Err("esta carpeta es demasiado general para borrarla de forma segura".into());
    }
    let mut raw = custom_raw();
    let s = p.to_string_lossy().into_owned();
    if !raw.iter().any(|x| x.eq_ignore_ascii_case(&s)) {
        raw.push(s);
        custom_save(&raw)?;
    }
    Ok(custom_list())
}

pub fn custom_remove(path: &str) -> Result<Vec<String>, String> {
    let mut raw = custom_raw();
    raw.retain(|x| !x.eq_ignore_ascii_case(path));
    custom_save(&raw)?;
    Ok(custom_list())
}

fn cats() -> Vec<Cat> {
    let env = |v: &str| PathBuf::from(std::env::var(v).unwrap_or_default());
    let (la, ad, pd, win) = (env("LocalAppData"), env("AppData"), env("ProgramData"), env("SystemRoot"));

    let mut browsers = vec![];
    for ud in [la.join(r"Google\Chrome\User Data"), la.join(r"Microsoft\Edge\User Data"), la.join(r"BraveSoftware\Brave-Browser\User Data")] {
        for p in profiles(&ud) {
            let n = p.file_name().unwrap_or_default().to_string_lossy().into_owned();
            if n == "Default" || n.starts_with("Profile ") {
                browsers.extend(["Cache", "Code Cache", "GPUCache"].iter().map(|c| plain(p.join(c))));
            }
        }
        browsers.extend(["ShaderCache", "GrShaderCache"].iter().map(|c| plain(ud.join(c))));
    }
    // Firefox-style browsers keep `cache2` inside each profile.
    for ff in [r"Mozilla\Firefox\Profiles", r"Waterfox\Profiles", r"Floorp\Profiles", r"librewolf\Profiles", r"Moonchild Productions\Pale Moon\Profiles"] {
        browsers.extend(profiles(&la.join(ff)).into_iter().map(|p| plain(p.join("cache2"))));
    }
    // Opera and Vivaldi are Chromium-based too; Opera keeps its profile directly in its folder.
    for ud in [la.join(r"Vivaldi\User Data"), la.join(r"Opera Software\Opera Stable"), la.join(r"Opera Software\Opera GX Stable")] {
        for p in std::iter::once(ud.clone()).chain(profiles(&ud)) {
            browsers.extend(["Cache", "Code Cache", "GPUCache"].iter().map(|c| plain(p.join(c))));
        }
    }
    browsers.push(plain(la.join(r"Microsoft\Windows\INetCache"))); // Internet Explorer / legacy Edge
    let sysdrive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let chkdsk: Vec<Target> = (0..10).map(|i| plain(PathBuf::from(format!(r"{sysdrive}\found.00{i}")))).filter(|t| t.dir.is_dir()).collect();

    vec![
        Cat { id: "temp", name: "Archivos temporales", desc: "Carpeta Temp del usuario y de Windows (solo lo de más de un día)", targets: vec![aged(env("TEMP")), aged(win.join("Temp"))] },
        Cat { id: "thumbs", name: "Caché de miniaturas", desc: "Windows las regenera al abrir carpetas con imágenes", targets: vec![files(la.join(r"Microsoft\Windows\Explorer"), "thumbcache_")] },
        Cat { id: "browsers", name: "Caché de navegadores", desc: "Chrome, Edge, Brave y Firefox. No toca contraseñas, cookies ni historial", targets: browsers },
        Cat { id: "gpu", name: "Caché de shaders de la GPU", desc: "NVIDIA, AMD y DirectX; se vuelve a generar al jugar", targets: vec![plain(la.join("D3DSCache")), plain(la.join(r"NVIDIA\DXCache")), plain(la.join(r"NVIDIA\GLCache")), plain(la.join(r"AMD\DxCache"))] },
        Cat {
            id: "apps",
            name: "Caché de aplicaciones",
            desc: "Discord, Visual Studio Code y Steam (web)",
            targets: ["Cache", "Code Cache", "GPUCache"].iter().map(|c| plain(ad.join("discord").join(c)))
                .chain(["Cache", "CachedData", "GPUCache"].iter().map(|c| plain(ad.join("Code").join(c))))
                .chain([plain(la.join(r"Steam\htmlcache"))])
                .collect(),
        },
        Cat { id: "dev", name: "Cachés de desarrollo", desc: "npm y pip; se vuelven a descargar al instalar paquetes", targets: vec![plain(la.join("npm-cache")), plain(la.join(r"pip\cache"))] },
        Cat {
            id: "dumps",
            name: "Informes y volcados de errores",
            desc: "Crash dumps, informes de Windows y MEMORY.DMP",
            targets: vec![plain(la.join("CrashDumps")), plain(pd.join(r"Microsoft\Windows\WER\ReportQueue")), plain(pd.join(r"Microsoft\Windows\WER\ReportArchive")), plain(win.join("Minidump")), files(win.clone(), "MEMORY.DMP")],
        },
        Cat { id: "logs", name: "Archivos de registro de Windows", desc: "Registros de CBS, DISM y Windows Update; Windows crea otros nuevos", targets: vec![plain(win.join(r"Logs\CBS")), plain(win.join(r"Logs\DISM")), plain(win.join(r"Logs\WindowsUpdate"))] },
        Cat { id: "iconcache", name: "Caché de iconos", desc: "Windows la regenera al abrir el Explorador", targets: vec![files(la.clone(), "IconCache.db"), files(la.join(r"Microsoft\Windows\Explorer"), "iconcache_")] },
        Cat { id: "chkdsk", name: "Fragmentos de Chkdsk", desc: "Archivos .chk recuperados en las carpetas found.000 del disco del sistema", targets: chkdsk },
        Cat { id: "prefetch", name: "Datos antiguos de Prefetch", desc: "De más de 7 días; ayuda a abrir programas más rápido, Windows lo recrea al usarlos", targets: vec![Target { dir: win.join("Prefetch"), prefix: None, min_age: 7 * 86_400 }] },
        Cat { id: "fontcache", name: "Caché de fuentes", desc: "Windows la reconstruye al iniciar; algunos archivos pueden estar en uso", targets: vec![plain(win.join(r"ServiceProfiles\LocalService\AppData\Local\FontCache"))] },
        Cat { id: "store", name: "Caché de la Microsoft Store", desc: "Descargas y datos temporales de la Tienda", targets: vec![plain(la.join(r"Packages\Microsoft.WindowsStore_8wekyb3d8bbwe\LocalCache"))] },
        Cat { id: "recent", name: "Documentos recientes", desc: "Historial de archivos abiertos hace poco (privacidad)", targets: vec![plain(ad.join(r"Microsoft\Windows\Recent"))] },
        Cat { id: "custom", name: "Personalizado", desc: "Carpetas que añadas tú: se vacía su contenido y la carpeta se conserva", targets: custom_list().into_iter().map(|d| plain(PathBuf::from(d))).collect() },
        Cat { id: "wu", name: "Descargas de Windows Update", desc: "Instaladores de actualizaciones ya descargados; se bajan de nuevo si hacen falta", targets: vec![plain(win.join(r"SoftwareDistribution\Download"))] },
    ]
    .into_iter()
    .map(|mut c| {
        // A relative path (from an unset env var) would resolve against the working directory: drop it.
        c.targets.retain(|t| t.dir.is_absolute() && t.dir.components().count() >= 3);
        c
    })
    .collect()
}

fn entries(t: &Target) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(&t.dir) else { return vec![] };
    let cutoff = SystemTime::now().checked_sub(Duration::from_secs(t.min_age));
    rd.flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let Ok(m) = e.metadata() else { return false };
            t.prefix.is_none_or(|p| name.starts_with(p) && m.is_file())
                && (t.min_age == 0 || m.modified().ok().zip(cutoff).is_some_and(|(m, c)| m < c))
        })
        .map(|e| e.path())
        .collect()
}

/// (bytes, files) under `p`, not following symlinks.
fn measure(p: &Path) -> (u64, u64) {
    match fs::symlink_metadata(p) {
        Ok(m) if m.is_dir() => fs::read_dir(p).map(|d| d.flatten().fold((0, 0), |a, e| { let b = measure(&e.path()); (a.0 + b.0, a.1 + b.1) })).unwrap_or((0, 0)),
        Ok(m) => (m.len(), 1),
        _ => (0, 0),
    }
}

/// The Recycle Bin isn't a folder to walk: Windows is asked how full it is and to empty it.
fn recycle_bin() -> (u64, u64) {
    use windows::{core::PCWSTR, Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO}};
    let mut info = SHQUERYRBINFO { cbSize: size_of::<SHQUERYRBINFO>() as u32, ..Default::default() };
    match unsafe { SHQueryRecycleBinW(PCWSTR::null(), &mut info) } {
        Ok(()) => (info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64),
        Err(_) => (0, 0),
    }
}

fn empty_recycle_bin() -> bool {
    use windows::{core::PCWSTR, Win32::UI::Shell::{SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND}};
    unsafe { SHEmptyRecycleBinW(None, PCWSTR::null(), SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND) }.is_ok()
}

pub fn scan(on: &dyn Fn(crate::Progress)) -> Vec<JunkCat> {
    let list = cats();
    let total = list.len() + 1; // + the Recycle Bin
    let mut out: Vec<JunkCat> = list
        .into_iter()
        .enumerate()
        .map(|(done, c)| {
            on(crate::Progress { done, total, id: c.id.into(), label: String::new() });
            let (size, files) = c.targets.iter().flat_map(entries).map(|p| measure(&p)).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
            JunkCat { id: c.id, name: c.name, desc: c.desc, size, files, on: !OFF_BY_DEFAULT.contains(&c.id) }
        })
        .collect();
    on(crate::Progress { done: total - 1, total, id: "recycle".into(), label: String::new() });
    let (size, files) = recycle_bin();
    out.push(JunkCat { id: "recycle", name: "Papelera de reciclaje", desc: "Vacía la papelera de todos los discos (se borra de forma definitiva)", size, files, on: false });
    on(crate::Progress { done: total, total, id: String::new(), label: String::new() });
    out
}

/// Deletes what it can and counts what it couldn't (files in use are simply left behind). `tick` is called before each file.
fn wipe(p: &Path, tick: &mut dyn FnMut(&Path)) -> (u64, u64) {
    let Ok(m) = fs::symlink_metadata(p) else { return (0, 0) };
    if m.is_dir() {
        let (mut freed, mut skipped) = (0, 0);
        for e in fs::read_dir(p).into_iter().flatten().flatten() {
            let (f, s) = wipe(&e.path(), tick);
            freed += f;
            skipped += s;
        }
        if fs::remove_dir(p).is_err() {
            skipped += 1;
        }
        return (freed, skipped);
    }
    tick(p);
    let mut perm = m.permissions();
    if perm.readonly() {
        #[allow(clippy::permissions_set_readonly_false)] // Windows: only clears the read-only attribute
        perm.set_readonly(false);
        let _ = fs::set_permissions(p, perm);
    }
    match fs::remove_file(p).or_else(|_| fs::remove_dir(p)) {
        Ok(()) => (m.len(), 0),
        Err(_) => (0, 1),
    }
}

/// Only ids from the built-in list are honoured; the UI never supplies paths.
pub fn clean(ids: &[String], on: &dyn Fn(crate::Progress)) -> JunkReport {
    let (mut freed, mut skipped) = (0, 0);
    // What will be removed is measured first, so progress can be counted in files.
    let jobs: Vec<(&str, Vec<(PathBuf, u64)>)> = cats()
        .into_iter()
        .filter(|c| ids.iter().any(|i| i == c.id))
        .map(|c| (c.id, c.targets.iter().flat_map(entries).map(|p| { let n = measure(&p).1.max(1); (p, n) }).collect()))
        .collect();
    let recycle = ids.iter().any(|i| i == "recycle");
    let total = jobs.iter().flat_map(|(_, es)| es.iter().map(|(_, n)| *n as usize)).sum::<usize>() + usize::from(recycle);
    let mut done = 0usize;
    for (id, entries) in &jobs {
        for (p, _) in entries {
            let (f, s) = wipe(p, &mut |q: &Path| {
                done += 1;
                on(crate::Progress { done, total, id: (*id).into(), label: q.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default() });
            });
            freed += f;
            skipped += s;
        }
    }
    if recycle {
        on(crate::Progress { done, total, id: "recycle".into(), label: String::new() });
        let (size, _) = recycle_bin();
        if empty_recycle_bin() {
            freed += size;
        } else {
            skipped += 1;
        }
    }
    on(crate::Progress { done: total, total, id: String::new(), label: String::new() });
    JunkReport { freed, skipped }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wipe_removes_nested_files_and_counts_bytes() {
        let d = std::env::temp_dir().join("vanish_junk_test");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("a/b")).unwrap();
        fs::write(d.join("a/b/x.bin"), [0u8; 100]).unwrap();
        fs::write(d.join("y.bin"), [0u8; 50]).unwrap();
        assert_eq!(measure(&d), (150, 2));
        let mut seen = 0;
        assert_eq!(wipe(&d, &mut |_| seen += 1), (150, 0));
        assert_eq!(seen, 2); // one tick per file
        assert!(!d.exists());
    }

    #[test]
    fn custom_folders_are_validated_and_stored() {
        let data = std::env::temp_dir().join("vanish_custom_data");
        let target = std::env::temp_dir().join("vanish_custom_target");
        let _ = fs::remove_dir_all(&data);
        let _ = fs::remove_dir_all(&target);
        fs::create_dir_all(&target).unwrap();
        std::env::set_var("VANISH_DATA", &data);
        assert!(custom_add(&std::env::var("SystemRoot").unwrap()).is_err()); // too general
        assert!(custom_add(r"C:\definitely\not\here").is_err()); // missing
        let list = custom_add(&format!("\"{}\"", target.display())).unwrap(); // quotes from "Copy as path" are fine
        assert_eq!(list, vec![target.to_string_lossy().into_owned()]);
        assert_eq!(custom_add(&target.to_string_lossy()).unwrap().len(), 1); // no duplicates
        fs::write(target.join("a.bin"), [0u8; 10]).unwrap();
        let c = scan(&|_| {}).into_iter().find(|c| c.id == "custom").unwrap();
        assert_eq!((c.size, c.files), (10, 1));
        assert_eq!(clean(&["custom".into()], &|_| {}).freed, 10);
        assert!(target.is_dir() && !target.join("a.bin").exists()); // contents go, the folder stays
        assert!(custom_remove(&target.to_string_lossy()).unwrap().is_empty());
        let _ = fs::remove_dir_all(&data);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn scan_measures_without_deleting() {
        for c in scan(&|_| {}) {
            println!("  {:<10} {:>12} bytes {:>7} files", c.id, c.size, c.files);
        }
    }

    #[test]
    fn every_target_is_absolute_and_ids_are_unique() {
        let c = cats();
        assert!(c.iter().all(|c| c.targets.iter().all(|t| t.dir.is_absolute())));
        let mut ids: Vec<_> = c.iter().map(|c| c.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), c.len());
    }
}
