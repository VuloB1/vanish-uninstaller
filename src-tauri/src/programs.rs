use serde::{Deserialize, Serialize};
use winreg::{enums::*, RegKey};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Program {
    /// `HIVE\path` of its Uninstall key.
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub install_location: String,
    pub uninstall: String,
    pub size_kb: u64,
    pub install_date: String,
    /// Icon path for registry programs; a ready-made data URL for Store apps.
    pub display_icon: String,
    /// Package family name; non-empty only for Microsoft Store apps.
    #[serde(default)]
    pub family: String,
}

const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL32: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

pub fn list() -> Vec<Program> {
    let mut out: Vec<Program> = Vec::new();
    for (hive, h, path) in [
        ("HKLM", HKEY_LOCAL_MACHINE, UNINSTALL),
        ("HKLM", HKEY_LOCAL_MACHINE, UNINSTALL32),
        ("HKCU", HKEY_CURRENT_USER, UNINSTALL),
    ] {
        let Ok(root) = RegKey::predef(h).open_subkey(path) else { continue };
        for sub in root.enum_keys().flatten() {
            let Ok(k) = root.open_subkey(&sub) else { continue };
            let s = |n: &str| k.get_value::<String, _>(n).unwrap_or_default();
            let name = s("DisplayName");
            // Skip hidden components and patches/updates, which aren't programs the user installed.
            if name.is_empty()
                || k.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1
                || !s("ParentKeyName").is_empty()
            {
                continue;
            }
            let mut uninstall = s("UninstallString");
            if uninstall.is_empty() && sub.starts_with('{') {
                uninstall = format!("msiexec /x {sub}");
            }
            let p = Program {
                id: format!(r"{hive}\{path}\{sub}"),
                version: s("DisplayVersion"),
                publisher: s("Publisher"),
                install_location: s("InstallLocation"),
                install_date: s("InstallDate"),
                display_icon: s("DisplayIcon"),
                family: String::new(),
                size_kb: k.get_value::<u32, _>("EstimatedSize").unwrap_or(0) as u64,
                uninstall,
                name,
            };
            if !out.iter().any(|o| o.name == p.name && o.version == p.version) {
                out.push(p);
            }
        }
    }
    out.sort_by_key(|p| p.name.to_lowercase());
    out
}
