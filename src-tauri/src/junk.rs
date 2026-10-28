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

#[derive(Serialize)]
pub struct JunkCat {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    size: u64,
    files: u64,
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
    browsers.extend(profiles(&la.join(r"Mozilla\Firefox\Profiles")).into_iter().map(|p| plain(p.join("cache2"))));

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
            t.prefix.map_or(true, |p| name.starts_with(p) && m.is_file())
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

pub fn scan() -> Vec<JunkCat> {
    cats()
        .into_iter()
        .map(|c| {
            let (size, files) = c.targets.iter().flat_map(entries).map(|p| measure(&p)).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
            JunkCat { id: c.id, name: c.name, desc: c.desc, size, files }
        })
        .collect()
}

/// Deletes what it can and counts what it couldn't (files in use are simply left behind).
fn wipe(p: &Path) -> (u64, u64) {
    let Ok(m) = fs::symlink_metadata(p) else { return (0, 0) };
    if m.is_dir() {
        let (mut freed, mut skipped) = (0, 0);
        for e in fs::read_dir(p).into_iter().flatten().flatten() {
            let (f, s) = wipe(&e.path());
            freed += f;
            skipped += s;
        }
        if fs::remove_dir(p).is_err() {
            skipped += 1;
        }
        return (freed, skipped);
    }
    let mut perm = m.permissions();
    if perm.readonly() {
        perm.set_readonly(false);
        let _ = fs::set_permissions(p, perm);
    }
    match fs::remove_file(p).or_else(|_| fs::remove_dir(p)) {
        Ok(()) => (m.len(), 0),
        Err(_) => (0, 1),
    }
}

/// Only ids from the built-in list are honoured; the UI never supplies paths.
pub fn clean(ids: &[String]) -> JunkReport {
    let (mut freed, mut skipped) = (0, 0);
    for c in cats().into_iter().filter(|c| ids.iter().any(|i| i == c.id)) {
        for p in c.targets.iter().flat_map(entries) {
            let (f, s) = wipe(&p);
            freed += f;
            skipped += s;
        }
    }
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
        assert_eq!(wipe(&d), (150, 0));
        assert!(!d.exists());
    }

    #[test]
    fn scan_measures_without_deleting() {
        for c in scan() {
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
