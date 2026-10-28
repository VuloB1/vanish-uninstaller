use crate::programs::Program;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use winreg::{enums::*, RegKey};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Leftover {
    pub id: String,
    pub program: String,
    pub kind: &'static str, // file | folder | reg
    pub path: String,
    pub confidence: &'static str, // high | medium | low
    pub reason: String,
    pub size: u64,
}

#[derive(Serialize)]
pub struct Report {
    pub moved: usize,
    /// Already removed by the program's own uninstaller between the scan and now.
    pub gone: usize,
    pub failed: Vec<String>,
    pub quarantine: String,
}

/// Last scan results. Deletion only accepts ids from here, never paths from the UI.
#[derive(Default)]
pub struct Scan(pub Mutex<HashMap<String, Leftover>>);

const NOISE: [&str; 10] = ["inc", "ltd", "llc", "corp", "corporation", "co", "gmbh", "limited", "software", "technologies"];
// Names that are never offered, whatever they match.
const PROTECTED: [&str; 14] = [
    "microsoft", "windows", "windowsapps", "commonfiles", "common", "packages", "programs", "temp", "classes",
    "policies", "wow6432node", "system32", "software", "clients",
];

pub fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// "7-Zip 23.01 (x64)" -> "7zip": drops parentheticals and bare version words.
pub fn name_key(name: &str) -> String {
    let (mut depth, mut s) = (0i32, String::new());
    for c in name.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ if depth == 0 => s.push(c),
            _ => {}
        }
    }
    let is_ver = |w: &&str| {
        let w = w.trim_start_matches(['v', 'V']);
        !w.is_empty() && w.chars().all(|c| c.is_ascii_digit() || c == '.')
    };
    let k = norm(&s.split_whitespace().filter(|w| !is_ver(w)).collect::<Vec<_>>().join(" "));
    if k.is_empty() { norm(name) } else { k }
}

fn pub_key(p: &str) -> String {
    p.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !NOISE.contains(w))
        .collect()
}

// shortcut: "contains" match needs a >=5 char key to limit false hits, upgrade to a per-program rules file.
fn hit(folder: &str, key: &str) -> Option<(&'static str, &'static str)> {
    let f = norm(folder);
    if key.is_empty() || PROTECTED.contains(&f.as_str()) {
        None
    } else if f == key {
        Some(("high", "Nombre coincide exactamente"))
    } else if key.len() >= 5 && f.contains(key) {
        Some(("medium", "Nombre parecido"))
    } else {
        None
    }
}

/// Ends every process running from inside `dir`, so its files can be moved afterwards.
pub fn kill_in(dir: &str) -> Result<(), String> {
    let d = Path::new(dir.trim_end_matches('\\'));
    if dir.is_empty() || !guard(d) {
        return Err("carpeta de instalación no válida".into());
    }
    // The path travels in an env var, never in the command text.
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", r"$d = $env:VANISH_DIR.TrimEnd('\') + '\'; Get-Process | Where-Object { $_.Path -and $_.Path.StartsWith($d, [StringComparison]::OrdinalIgnoreCase) } | Stop-Process -Force -ErrorAction SilentlyContinue"])
        .env("VANISH_DIR", d)
        .creation_flags(0x0800_0000)
        .status()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn guard(p: &Path) -> bool {
    let low = p.to_string_lossy().to_lowercase();
    let low = low.trim_end_matches('\\');
    let win = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into()).to_lowercase();
    let is_root = ["ProgramFiles", "ProgramFiles(x86)", "ProgramData", "AppData", "LocalAppData", "UserProfile", "SystemRoot"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .any(|r| r.trim_end_matches('\\').to_lowercase() == low);
    let protected_name = p.file_name().is_some_and(|n| PROTECTED.contains(&norm(&n.to_string_lossy()).as_str()));
    p.is_absolute()
        && p.components().count() >= 3
        && !low.starts_with(&win)
        && !low.contains(r"\windowsapps")
        && !low.ends_with(r":\users")
        && !is_root
        && !protected_name
}

fn size(p: &Path) -> u64 {
    match fs::symlink_metadata(p) {
        Ok(m) if m.is_dir() => fs::read_dir(p).map(|d| d.flatten().map(|e| size(&e.path())).sum()).unwrap_or(0),
        Ok(m) => m.len(),
        _ => 0,
    }
}

pub const SEP: &str = " → "; // joins a registry key and a value name in a leftover's path
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const RUN_KEYS: [(&str, winreg::HKEY, &str); 5] = [
    ("HKCU", HKEY_CURRENT_USER, RUN),
    ("HKCU", HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\RunOnce"),
    ("HKLM", HKEY_LOCAL_MACHINE, RUN),
    ("HKLM", HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\RunOnce"),
    ("HKLM", HKEY_LOCAL_MACHINE, r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run"),
];

fn tasks_dir() -> PathBuf {
    PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into())).join(r"System32\Tasks")
}

fn collect_tasks(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return }; // unreadable without admin: just yields fewer results
    for e in rd.flatten() {
        let (path, name) = (e.path(), e.file_name().to_string_lossy().to_lowercase());
        if path.is_dir() {
            if !(dir == tasks_dir() && name == "microsoft") {
                collect_tasks(&path, out);
            }
        } else {
            out.push(path);
        }
    }
}

/// Task files are XML, normally UTF-16 with a BOM.
fn read_task(p: &Path) -> String {
    let b = fs::read(p).unwrap_or_default();
    if b.starts_with(&[0xFF, 0xFE]) {
        String::from_utf16_lossy(&b[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>())
    } else {
        String::from_utf8_lossy(&b).into_owned()
    }
}

/// `HKCU\Software\...\Run → Name` -> (hive, subkey, value name)
fn split_value(path: &str) -> Option<(&str, &str, &str)> {
    let (key, name) = path.rsplit_once(SEP)?;
    let (hive, sub) = key.split_once('\\')?;
    Some((hive, sub, name))
}

fn hive(h: &str) -> RegKey {
    RegKey::predef(if h == "HKCU" { HKEY_CURRENT_USER } else { HKEY_LOCAL_MACHINE })
}

fn scan_one(p: &Program, all: &[Program]) -> Vec<Leftover> {
    let (nk, pk) = (name_key(&p.name), pub_key(&p.publisher));
    // A publisher folder is only offered when no other installed program shares that publisher.
    let shared = !pk.is_empty() && all.iter().any(|o| o.id != p.id && pub_key(&o.publisher) == pk);
    let mut out: Vec<Leftover> = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |kind: &'static str, path: String, confidence: &'static str, reason: &'static str, size: u64| {
        if seen.insert(path.to_lowercase()) {
            out.push(Leftover { id: String::new(), program: p.name.clone(), kind, path, confidence, reason: reason.into(), size });
        }
    };

    if !p.family.is_empty() {
        // Store apps live in the read-only WindowsApps; the only thing they leave behind is their data folder.
        let data = PathBuf::from(std::env::var("LocalAppData").unwrap_or_default()).join("Packages").join(&p.family);
        if data.is_dir() && guard(&data) {
            add("folder", data.to_string_lossy().into(), "high", "Datos de la app de la Store", size(&data));
        }
        return out;
    }

    let loc = Path::new(&p.install_location);
    // Lower-cased `<install folder>\`; the trailing `\` avoids matching `C:\FooBar` for `C:\Foo`.
    let prefix = (!p.install_location.is_empty() && guard(loc)).then(|| format!(r"{}\", p.install_location.trim_end_matches('\\').to_lowercase()));
    let points_here = |s: &str| prefix.as_ref().is_some_and(|x| s.to_lowercase().replace('"', "").contains(x.as_str()));
    if prefix.is_some() {
        if loc.is_dir() {
            add("folder", p.install_location.trim_end_matches('\\').into(), "high", "Carpeta de instalación", size(loc));
        }
        if let Ok(svcs) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SYSTEM\CurrentControlSet\Services") {
            for s in svcs.enum_keys().flatten() {
                let img: String = svcs.open_subkey(&s).and_then(|k| k.get_value("ImagePath")).unwrap_or_default();
                if points_here(&img) {
                    add("reg", format!(r"HKLM\SYSTEM\CurrentControlSet\Services\{s}"), "high", "Servicio de Windows del programa", 0);
                }
            }
        }
    }

    // Autostart entries: values (not keys) under Run/RunOnce, matched by what they launch or by their name.
    for (hive, h, key) in RUN_KEYS {
        let Ok(k) = RegKey::predef(h).open_subkey(key) else { continue };
        for name in k.enum_values().flatten().map(|(n, _)| n) {
            let data: String = k.get_value(&name).unwrap_or_default();
            let found = if points_here(&data) { Some(("high", "Inicio automático del programa")) } else { hit(&name, &nk) };
            if let Some((c, why)) = found {
                add("value", format!(r"{hive}\{key}{SEP}{name}"), c, why, 0);
            }
        }
    }

    // Scheduled tasks: by the command they run or by their name (Microsoft's own tasks are skipped).
    let mut tasks = Vec::new();
    collect_tasks(&tasks_dir(), &mut tasks);
    for f in tasks {
        let name = format!(r"\{}", f.strip_prefix(tasks_dir()).unwrap_or(&f).to_string_lossy());
        let fname = f.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let found = if points_here(&read_task(&f)) { Some(("high", "Tarea programada del programa")) } else { hit(&fname, &nk) };
        if let Some((c, why)) = found {
            add("task", name, c, why, 0);
        }
    }

    let env = |v: &str| std::env::var(v).ok().map(PathBuf::from);
    let (la, ad, pd) = (env("LocalAppData"), env("AppData"), env("ProgramData"));
    let start = r"Microsoft\Windows\Start Menu\Programs";
    let roots = [
        env("ProgramFiles"),
        env("ProgramFiles(x86)"),
        pd.clone(),
        ad.clone(),
        la.clone(),
        la.map(|x| x.join("Programs")),
        pd.map(|x| x.join(start)),
        ad.map(|x| x.join(start)),
    ];
    for root in roots.into_iter().flatten() {
        let Ok(rd) = fs::read_dir(&root) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            let is_dir = path.is_dir();
            let name = if is_dir { e.file_name() } else { path.file_stem().unwrap_or_default().to_owned() };
            let name = name.to_string_lossy();
            let found = hit(&name, &nk).or_else(|| {
                (!shared && !pk.is_empty() && norm(&name) == pk && !PROTECTED.contains(&pk.as_str()))
                    .then_some(("low", "Carpeta del editor (sin otras apps suyas)"))
            });
            if let (Some((c, why)), true) = (found, guard(&path)) {
                add(if is_dir { "folder" } else { "file" }, path.to_string_lossy().into(), c, why, size(&path));
            }
        }
    }

    for (hive, h, path) in [
        ("HKCU", HKEY_CURRENT_USER, "Software"),
        ("HKLM", HKEY_LOCAL_MACHINE, "SOFTWARE"),
        ("HKLM", HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node"),
    ] {
        let Ok(k) = RegKey::predef(h).open_subkey(path) else { continue };
        for s in k.enum_keys().flatten() {
            let found = hit(&s, &nk).or_else(|| {
                (!shared && !pk.is_empty() && norm(&s) == pk && !PROTECTED.contains(&pk.as_str()))
                    .then_some(("low", "Clave del editor (sin otras apps suyas)"))
            });
            if let Some((c, why)) = found {
                add("reg", format!(r"{hive}\{path}\{s}"), c, why, 0);
            }
        }
    }

    if let Some((hive, sub)) = p.id.split_once('\\') {
        let h = if hive == "HKCU" { HKEY_CURRENT_USER } else { HKEY_LOCAL_MACHINE };
        if RegKey::predef(h).open_subkey(sub).is_ok() {
            add("reg", p.id.clone(), "high", "Entrada de desinstalación", 0);
        }
    }

    out
}

/// Scans several programs at once; a path found for more than one program is listed once.
pub fn scan(ps: &[Program], all: &[Program]) -> Vec<Leftover> {
    let mut seen = HashSet::new();
    let mut out: Vec<Leftover> = ps.iter().flat_map(|p| scan_one(p, all)).filter(|l| seen.insert(l.path.to_lowercase())).collect();
    let rank = |c: &str| ["high", "medium", "low"].iter().position(|x| *x == c).unwrap_or(3);
    out.sort_by(|a, b| rank(a.confidence).cmp(&rank(b.confidence)).then(a.path.cmp(&b.path)));
    for (i, l) in out.iter_mut().enumerate() {
        l.id = format!("l{i}");
    }
    out
}

fn app_dir() -> PathBuf {
    PathBuf::from(std::env::var("LocalAppData").unwrap_or_else(|_| r"C:\".into())).join("Vanish")
}

/// Moves to a quarantine on the same volume (rename is instant and keeps the data recoverable).
fn quarantine(path: &Path, ts: u64, n: usize) -> Result<(), String> {
    let drive = path.components().next().map(|c| c.as_os_str().to_string_lossy().into_owned()).unwrap_or_default();
    let sys = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let base = if drive.eq_ignore_ascii_case(&sys) { app_dir().join("quarantine") } else { PathBuf::from(format!(r"{drive}\.vanish-quarantine")) };
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let dest = base.join(ts.to_string()).join(format!("{n}_{name}"));
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::rename(path, dest).map_err(|e| e.to_string())
}

fn backup_dir(ts: u64) -> Result<PathBuf, String> {
    let dir = app_dir().join("backups").join(ts.to_string());
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn delete_reg(path: &str, ts: u64, n: usize, done: &mut HashSet<String>) -> Result<(), String> {
    let (h, sub) = path.split_once('\\').ok_or("clave inválida")?;
    backup_key(path, ts, n, done)?;
    hive(h).delete_subkey_all(sub).map_err(|e| e.to_string())
}

fn delete_value(path: &str, ts: u64, n: usize, done: &mut HashSet<String>) -> Result<(), String> {
    let (h, sub, name) = split_value(path).ok_or("valor inválido")?;
    backup_key(&format!(r"{h}\{sub}"), ts, n, done)?;
    hive(h).open_subkey_with_flags(sub, KEY_SET_VALUE).and_then(|k| k.delete_value(name)).map_err(|e| e.to_string())
}

fn delete_task(name: &str, ts: u64, n: usize) -> Result<(), String> {
    let file = tasks_dir().join(name.trim_start_matches('\\'));
    fs::copy(&file, backup_dir(ts)?.join(format!("{n}.task.xml"))).map_err(|e| e.to_string())?; // never delete without a backup
    let ok = Command::new("schtasks").args(["/delete", "/tn", name, "/f"]).creation_flags(0x0800_0000).status().map(|s| s.success()).unwrap_or(false);
    if ok { Ok(()) } else { Err("schtasks no pudo eliminar la tarea (¿falta administrador?)".into()) }
}

/// Exports `path` to a .reg file once per batch: many values of one key (e.g. SharedDLLs) share a single backup.
fn backup_key(path: &str, ts: u64, n: usize, done: &mut HashSet<String>) -> Result<(), String> {
    if done.contains(&path.to_lowercase()) {
        return Ok(());
    }
    let file = backup_dir(ts)?.join(format!("{n}.reg"));
    let ok = Command::new("reg")
        .args(["export", path])
        .arg(&file)
        .arg("/y")
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return Err("no se pudo hacer la copia de seguridad de la clave".into()); // never delete without a backup
    }
    done.insert(path.to_lowercase());
    Ok(())
}

/// Removes quarantine batches older than 7 days (batch folders are named by their unix timestamp).
// shortcut: .reg backups are kept forever (tiny); add a retention policy if they ever pile up.
pub fn purge_old() {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mut bases = vec![app_dir().join("quarantine")];
    bases.extend(('A'..='Z').map(|d| PathBuf::from(format!(r"{d}:\.vanish-quarantine"))));
    for base in bases {
        let Ok(rd) = fs::read_dir(&base) else { continue };
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().parse::<u64>().is_ok_and(|ts| now.saturating_sub(ts) > 7 * 86400) {
                let _ = fs::remove_dir_all(e.path());
            }
        }
    }
}

pub fn delete(state: &Scan, ids: Vec<String>) -> Report {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let items = state.0.lock().unwrap();
    let (mut moved, mut gone, mut failed) = (0, 0, Vec::new());
    let mut backed_up = HashSet::new();
    for (n, id) in ids.iter().enumerate() {
        let Some(l) = items.get(id) else { continue };
        let exists = match l.kind {
            "reg" => l.path.split_once('\\').is_some_and(|(h, sub)| hive(h).open_subkey(sub).is_ok()),
            "value" => split_value(&l.path).is_some_and(|(h, sub, name)| hive(h).open_subkey(sub).and_then(|k| k.get_raw_value(name)).is_ok()),
            "task" => tasks_dir().join(l.path.trim_start_matches('\\')).exists(),
            _ => fs::symlink_metadata(&l.path).is_ok(),
        };
        if !exists {
            gone += 1;
            continue;
        }
        let r = match l.kind {
            "reg" => delete_reg(&l.path, ts, n, &mut backed_up),
            "value" => delete_value(&l.path, ts, n, &mut backed_up),
            "task" => delete_task(&l.path, ts, n),
            _ if guard(Path::new(&l.path)) => quarantine(Path::new(&l.path), ts, n),
            _ => Err("ruta protegida".into()),
        };
        match r {
            Ok(()) => moved += 1,
            Err(e) => failed.push(format!("{}: {e}", l.path)),
        }
    }
    Report { moved, gone, failed, quarantine: app_dir().to_string_lossy().into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        assert_eq!(name_key("7-Zip 23.01 (x64 edition)"), "7zip");
        assert_eq!(name_key("Python 3.12.1"), "python");
        assert_eq!(pub_key("Igor Pavlov, Inc."), "igorpavlov");
    }

    #[test]
    fn matching() {
        assert_eq!(hit("Notepad++", "notepad").map(|h| h.0), Some("high"));
        assert_eq!(hit("FooBarApp Updater", "foobarapp").map(|h| h.0), Some("medium"));
        assert_eq!(hit("Foo", "foobarapp"), None);
        assert_eq!(hit("Microsoft", "microsoft"), None);
    }

    #[test]
    fn splits_value_paths() {
        assert_eq!(split_value(r"HKCU\Software\X\Run → Foo Bar"), Some(("HKCU", r"Software\X\Run", "Foo Bar")));
        assert_eq!(split_value(r"HKCU\Software\X"), None);
    }

    #[test]
    fn reads_utf16_task_files() {
        let f = std::env::temp_dir().join("vanish_task_test.xml");
        let mut b = vec![0xFF, 0xFE];
        b.extend("<Command>C:\\Foo\\a.exe</Command>".encode_utf16().flat_map(|u| u.to_le_bytes()));
        fs::write(&f, b).unwrap();
        assert!(read_task(&f).contains(r"C:\Foo\a.exe"));
        let _ = fs::remove_file(&f);
    }

    #[test]
    fn guards() {
        assert!(!guard(Path::new(r"C:\Windows\System32\foo")));
        assert!(!guard(Path::new(r"C:\Program Files")));
        assert!(!guard(Path::new(r"C:\Program Files\Common Files")));
        assert!(!guard(Path::new(r"C:\Program Files\WindowsApps\Foo_1.0_x64__abc")));
        assert!(guard(Path::new(r"D:\Games\Foo")));
        assert!(kill_in("").is_err() && kill_in(r"C:\Windows").is_err() && kill_in(r"C:\Program Files").is_err());
    }
}
