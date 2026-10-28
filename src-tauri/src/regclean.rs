use crate::{
    icon::{command_exe, expand},
    leftovers::{Leftover, RUN_KEYS, SEP},
};
use std::path::Path;
use winreg::{enums::*, RegKey};

/// True only for an absolute local path (`C:\...`) that doesn't exist. Anything else (bare names, UNC, odd
/// commands) is treated as "can't tell" and left alone.
fn missing(p: &str) -> bool {
    missing_view(p, false)
}

/// `%ProgramFiles%` and `%CommonProgramFiles%` mean the (x86) folders to 32-bit software.
fn to_x86(s: &str) -> String {
    let mut s = s.to_string();
    for (from, to) in [("%commonprogramfiles%", "%CommonProgramFiles(x86)%"), ("%programfiles%", "%ProgramFiles(x86)%")] {
        while let Some(i) = s.to_ascii_lowercase().find(from) {
            s.replace_range(i..i + from.len(), to);
        }
    }
    s
}

/// `wow64`: the value was read from the 32-bit registry view, so resolve its environment variables as 32-bit code would.
fn missing_view(p: &str, wow64: bool) -> bool {
    let p = if wow64 { to_x86(p.trim()) } else { p.trim().to_string() };
    let p = expand(&p).trim_matches('"').to_string();
    let b = p.as_bytes();
    // `Microsoft Shared` (Ink, DAO...) is Windows/Office plumbing and partly redirected by WOW64: not ours to judge.
    if p.to_ascii_lowercase().contains(r"\microsoft shared\") {
        return false;
    }
    b.len() > 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\' && !Path::new(&p).exists() && !syswow64(&p).is_some_and(|a| Path::new(&a).exists())
}

/// 32-bit programs see `\System32\` as `\SysWOW64\`, so a path that looks missing may just live there.
fn syswow64(p: &str) -> Option<String> {
    let i = p.to_ascii_lowercase().find(r"\system32\")?;
    Some(format!(r"{}\syswow64\{}", &p[..i], &p[i + 10..]))
}

/// The file a COM server value points at: `"C:\x\a.dll"`, `C:\x\a.dll`, or `C:\x\a.exe /Embedding`.
fn server_path(v: &str) -> String {
    let v = v.trim();
    if let Some(rest) = v.strip_prefix('"') {
        return rest.split('"').next().unwrap_or_default().to_string();
    }
    let low = v.to_ascii_lowercase();
    match [".dll", ".exe", ".ocx"].iter().filter_map(|e| low.find(e).map(|i| i + 4)).min() {
        Some(end) => v[..end].to_string(),
        None => v.to_string(),
    }
}

const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL32: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
const APP_PATHS: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";
const APP_PATHS32: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths";
const SHARED: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\SharedDLLs";
const SHARED32: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\SharedDLLs";
const MUI: &str = r"Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache";
const APPROVED: [&str; 2] = [
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32",
];
const FIREWALL: &str = r"SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\FirewallRules";

const G_UNINSTALL: &str = "Programas ya eliminados (entradas de desinstalación huérfanas)";
const G_APPPATHS: &str = "Rutas de aplicación rotas";
const G_SHARED: &str = "DLL compartidas que ya no existen";
const G_RUN: &str = "Inicio automático roto";
const G_APPROVED: &str = "Estado de inicio automático sin programa";
const G_MUI: &str = "Historial de programas que ya no existen";
const G_COM: &str = "Componentes COM rotos (revisar)";
const G_FIREWALL: &str = "Reglas de firewall de programas inexistentes (revisar)";

/// Registry entries that point at files that no longer exist. Every item is a whole key or a single value, and
/// goes through the same backup-then-delete path as leftovers.
pub fn scan() -> Vec<Leftover> {
    let mut out: Vec<Leftover> = Vec::new();
    // "high" items are pre-selected in the UI; "medium" ones (COM, firewall) are listed but left for the user to tick.
    let mut push = |group: &str, confidence: &'static str, kind: &'static str, path: String, why: String| {
        out.push(Leftover { id: String::new(), program: group.into(), kind, path, confidence, reason: why, size: 0 });
    };

    // Uninstall entries whose uninstaller AND install folder are both gone. MSI entries are skipped (no exe to check).
    for (hive, h, path) in [("HKLM", HKEY_LOCAL_MACHINE, UNINSTALL), ("HKLM", HKEY_LOCAL_MACHINE, UNINSTALL32), ("HKCU", HKEY_CURRENT_USER, UNINSTALL)] {
        let Ok(root) = RegKey::predef(h).open_subkey(path) else { continue };
        for sub in root.enum_keys().flatten() {
            let Ok(k) = root.open_subkey(&sub) else { continue };
            let s = |n: &str| k.get_value::<String, _>(n).unwrap_or_default();
            let name = s("DisplayName");
            if name.is_empty() || k.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1 {
                continue;
            }
            let exes: Vec<String> = [s("UninstallString"), s("QuietUninstallString")].iter().filter_map(|c| command_exe(c)).collect();
            let loc = s("InstallLocation");
            let loc_alive = !loc.is_empty() && !missing(&loc);
            if !exes.is_empty() && exes.iter().all(|e| missing(e)) && !loc_alive {
                push(G_UNINSTALL, "high", "reg", format!(r"{hive}\{path}\{sub}"), format!("{name} — falta {}", exes[0]));
            }
        }
    }

    for (hive, h, path) in [("HKLM", HKEY_LOCAL_MACHINE, APP_PATHS), ("HKLM", HKEY_LOCAL_MACHINE, APP_PATHS32), ("HKCU", HKEY_CURRENT_USER, APP_PATHS)] {
        let Ok(root) = RegKey::predef(h).open_subkey(path) else { continue };
        for sub in root.enum_keys().flatten() {
            let target: String = root.open_subkey(&sub).and_then(|k| k.get_value("")).unwrap_or_default();
            if missing_view(&target, path.contains("WOW6432Node")) {
                push(G_APPPATHS, "high", "reg", format!(r"{hive}\{path}\{sub}"), format!("{sub} — falta {}", target.trim_matches('"')));
            }
        }
    }

    for path in [SHARED, SHARED32] {
        let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) else { continue };
        for name in k.enum_values().flatten().map(|(n, _)| n) {
            if missing(&name) {
                push(G_SHARED, "high", "value", format!(r"HKLM\{path}{SEP}{name}"), "El archivo ya no existe".into());
            }
        }
    }

    let mut run_names = std::collections::HashSet::new();
    for (hive, h, key) in RUN_KEYS {
        let Ok(k) = RegKey::predef(h).open_subkey(key) else { continue };
        for name in k.enum_values().flatten().map(|(n, _)| n) {
            run_names.insert(name.to_lowercase());
            let data: String = k.get_value(&name).unwrap_or_default();
            if let Some(exe) = command_exe(&data).filter(|e| missing(e)) {
                push(G_RUN, "high", "value", format!(r"{hive}\{key}{SEP}{name}"), format!("Apunta a un archivo que no existe: {exe}"));
            }
        }
    }

    // "Enabled/disabled" flags Windows keeps for startup entries; with no matching Run value they're just stale state.
    for (hive, h) in [("HKCU", HKEY_CURRENT_USER), ("HKLM", HKEY_LOCAL_MACHINE)] {
        for key in APPROVED {
            let Ok(k) = RegKey::predef(h).open_subkey(key) else { continue };
            for name in k.enum_values().flatten().map(|(n, _)| n) {
                if !run_names.contains(&name.to_lowercase()) {
                    push(G_APPROVED, "high", "value", format!(r"{hive}\{key}{SEP}{name}"), "No hay ninguna entrada de inicio con este nombre".into());
                }
            }
        }
    }

    // Windows' cache of program display names; harmless, but it keeps entries for programs that are long gone.
    if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(MUI) {
        for name in k.enum_values().flatten().map(|(n, _)| n) {
            let file = name.trim_end_matches(".FriendlyAppName").trim_end_matches(".ApplicationCompany");
            if missing(file) {
                push(G_MUI, "high", "value", format!(r"HKCU\{MUI}{SEP}{name}"), format!("Falta {file}"));
            }
        }
    }

    // COM classes whose server file is gone. Anything under the Windows folder is Windows' business and is skipped.
    let windir = expand("%SystemRoot%").to_lowercase();
    for root in [r"SOFTWARE\Classes\CLSID", r"SOFTWARE\WOW6432Node\Classes\CLSID"] {
        let Ok(clsids) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(root) else { continue };
        for guid in clsids.enum_keys().flatten() {
            for server in ["InprocServer32", "LocalServer32"] {
                let v: String = clsids.open_subkey(format!(r"{guid}\{server}")).and_then(|k| k.get_value("")).unwrap_or_default();
                let file = server_path(&v);
                if !file.is_empty() && !expand(&file).to_lowercase().starts_with(&windir) && missing_view(&file, root.contains("WOW6432Node")) {
                    push(G_COM, "medium", "reg", format!(r"HKLM\{root}\{guid}"), format!("Falta {file}"));
                    break;
                }
            }
        }
    }

    if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(FIREWALL) {
        for name in k.enum_values().flatten().map(|(n, _)| n) {
            let rule: String = k.get_value(&name).unwrap_or_default();
            if let Some(app) = rule.split('|').find_map(|f| f.strip_prefix("App=")).filter(|a| missing(a)) {
                push(G_FIREWALL, "medium", "value", format!(r"HKLM\{FIREWALL}{SEP}{name}"), format!("Falta {app}"));
            }
        }
    }

    for (i, l) in out.iter_mut().enumerate() {
        l.id = format!("r{i}"); // distinct from leftover ids ("l..") so a stale list can never hit the wrong item
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_absolute_missing_local_paths_count() {
        assert!(missing(r"C:\definitely\not\here\a.exe"));
        assert!(missing(r#""C:\definitely\not\here\a.exe""#));
        assert!(!missing(r"C:\Windows\System32\notepad.exe")); // exists
        assert!(!missing(r"C:\Windows\system32\olepro32.dll")); // 32-bit DLL: only exists in SysWOW64
        assert!(!missing(r"C:\Program Files\Common Files\Microsoft Shared\ink\x.dll")); // Microsoft Shared: not judged
        assert!(!missing("notepad.exe")); // not absolute: can't tell
        assert!(!missing(r"\\server\share\a.exe")); // network: can't tell
        assert!(!missing(""));
    }

    #[test]
    fn x86_view_uses_x86_folders() {
        assert_eq!(to_x86(r"%CommonProgramFiles%\Microsoft Shared\a.dll"), r"%CommonProgramFiles(x86)%\Microsoft Shared\a.dll");
        assert_eq!(to_x86(r"%PROGRAMFILES%\X\a.dll"), r"%ProgramFiles(x86)%\X\a.dll");
        assert!(!missing_view(r"%CommonProgramFiles%\Microsoft Shared\DAO\dao360.dll", true)); // exists only under (x86)
    }

    #[test]
    fn server_paths_are_extracted() {
        assert_eq!(server_path(r#""C:\a b\x.dll""#), r"C:\a b\x.dll");
        assert_eq!(server_path(r"C:\a\x.exe /Embedding"), r"C:\a\x.exe");
        assert_eq!(server_path(r"C:\a b\x.DLL"), r"C:\a b\x.DLL");
        assert_eq!(server_path(""), "");
    }

    #[test]
    fn scan_runs_and_ids_use_r_prefix() {
        let found = scan();
        assert!(found.iter().all(|l| l.id.starts_with('r')));
        println!("registry issues on this machine: {}", found.len());
        for l in &found {
            println!("  [{}|{}] {} :: {}", l.program, l.confidence, l.path, l.reason);
        }
    }
}
