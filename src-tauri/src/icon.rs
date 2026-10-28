use crate::programs::Program;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{ffi::c_void, mem::size_of, path::Path};
use winreg::{enums::*, RegKey};
use windows::{
    core::HSTRING,
    Win32::{
        Graphics::Gdi::*,
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        UI::{
            Shell::{ExtractIconExW, SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON},
            WindowsAndMessaging::*,
        },
    },
};

/// The executable named by a command line (`"C:\x\a.exe" /s` or `C:\x\a.exe /s`), env vars expanded.
pub fn command_exe(cmd: &str) -> Option<String> {
    let c = expand(cmd.trim());
    if let Some(rest) = c.strip_prefix('"') {
        return rest.split_once('"').map(|(a, _)| a.to_string());
    }
    let end = c.to_ascii_lowercase().find(".exe")? + 4;
    Some(c[..end].to_string())
}

pub fn expand(s: &str) -> String {
    let mut out = String::new();
    let mut parts = s.split('%');
    out.push_str(parts.next().unwrap_or(""));
    while let Some(var) = parts.next() {
        match (std::env::var(var), parts.next()) {
            (Ok(v), rest) => {
                out.push_str(&v);
                out.push_str(rest.unwrap_or(""));
            }
            (Err(_), rest) => {
                out.push_str(&format!("%{var}%"));
                out.push_str(rest.unwrap_or(""));
            }
        }
    }
    out
}

/// `path`, `"path"` or `path,index` -> an existing *file* (a folder or a generic system tool is no use) and its icon index.
fn existing(raw: &str) -> Option<(String, i32)> {
    let raw = expand(raw.trim());
    let (mut path, idx) = if let Some(rest) = raw.strip_prefix('"') {
        let (q, tail) = rest.split_once('"')?;
        (q.to_string(), tail.trim_start_matches(',').trim().parse().unwrap_or(0))
    } else {
        match raw.rsplit_once(',') {
            Some((a, b)) if b.trim().parse::<i32>().is_ok() => (a.to_string(), b.trim().parse().unwrap()),
            _ => (raw.clone(), 0),
        }
    };
    if !Path::new(&path).is_file() {
        let end = path.to_ascii_lowercase().find(".exe")? + 4; // "unins.exe /S" -> "unins.exe"
        path.truncate(end);
    }
    let name = Path::new(&path).file_name()?.to_string_lossy().to_lowercase();
    let generic = ["msiexec.exe", "rundll32.exe", "sdbinst.exe", "cmd.exe", "powershell.exe"];
    (Path::new(&path).is_file() && !generic.contains(&name.as_str())).then_some((path, idx))
}

/// `{4B0BC91B-29A6-...}` -> the "packed" form Windows Installer uses as a registry key name.
fn pack_guid(g: &str) -> Option<String> {
    let g = g.trim_matches(|c| c == '{' || c == '}');
    let p: Vec<&str> = g.split('-').collect();
    if p.iter().map(|s| s.len()).collect::<Vec<_>>() != [8, 4, 4, 4, 12] {
        return None;
    }
    let rev = |s: &str| s.chars().rev().collect::<String>();
    let swap = |s: &str| s.as_bytes().chunks(2).flat_map(|c| [c[1] as char, c[0] as char]).collect::<String>();
    Some(format!("{}{}{}{}{}", rev(p[0]), rev(p[1]), rev(p[2]), swap(p[3]), swap(p[4])))
}

/// MSI products can name an icon in `Installer\Products\<packed guid>\ProductIcon`.
fn msi_icon(uninstall: &str) -> Option<(String, i32)> {
    let (s, e) = (uninstall.find('{')?, uninstall.find('}')?);
    let key = format!(r"SOFTWARE\Classes\Installer\Products\{}", pack_guid(uninstall.get(s..=e)?)?);
    let icon: String = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(key).ok()?.get_value("ProductIcon").ok()?;
    existing(&icon)
}

/// The program's own executable somewhere in `dir` (two levels deep, so `app-1.0\Discord.exe` is found). Installers,
/// updaters and helpers are skipped; a name match wins, otherwise the biggest exe, but only if it scores `min_score`.
fn find_exe(dir: &Path, name: &str, min_score: u8) -> Option<(String, i32)> {
    fn collect(dir: &Path, depth: u8, out: &mut Vec<(std::path::PathBuf, u64)>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let (p, Ok(m)) = (e.path(), e.metadata()) else { continue };
            if m.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")) {
                out.push((p, m.len()));
            } else if m.is_dir() && depth > 0 && out.len() < 300 {
                collect(&p, depth - 1, out);
            }
        }
    }
    if !dir.is_dir() {
        return None;
    }
    let key = crate::leftovers::name_key(name);
    let mut exes = Vec::new();
    collect(dir, 2, &mut exes);
    let skip = ["unins", "uninst", "setup", "update", "crash", "helper", "install", "report"];
    exes.into_iter()
        .filter_map(|(p, size)| {
            let stem = p.file_stem()?.to_string_lossy().to_lowercase();
            if skip.iter().any(|s| stem.contains(s)) {
                return None;
            }
            let n = crate::leftovers::norm(&stem);
            let score = if !key.is_empty() && n == key { 3 } else if !key.is_empty() && (n.contains(&key) || (n.len() >= 4 && key.contains(&n))) { 2 } else { 1 };
            (score >= min_score).then(|| ((score, size), p))
        })
        .max_by_key(|(rank, _)| *rank)
        .map(|(_, p)| (p.to_string_lossy().into_owned(), 0))
}

/// Windows Installer keeps a per-product folder, sometimes holding an icon-bearing file for Add/Remove Programs.
fn installer_cache_icon(uninstall: &str) -> Option<Vec<u8>> {
    let (s, e) = (uninstall.find('{')?, uninstall.find('}')?);
    let dir = Path::new(&expand("%SystemRoot%")).join("Installer").join(uninstall.get(s..=e)?);
    std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_file()).take(6).find_map(|p| png(&p.to_string_lossy(), 0))
}

/// Start Menu shortcuts whose name matches the program; Explorer's own icon for the shortcut is what the user knows.
fn shortcut_icon(name: &str) -> Option<Vec<u8>> {
    fn collect(dir: &Path, depth: u8, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() && depth > 0 {
                collect(&p, depth - 1, out);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")) {
                out.push(p);
            }
        }
    }
    let key = crate::leftovers::name_key(name);
    if key.len() < 3 {
        return None;
    }
    let start = r"Microsoft\Windows\Start Menu\Programs";
    let mut lnks = Vec::new();
    for var in ["ProgramData", "AppData"] {
        collect(&Path::new(&std::env::var(var).ok()?).join(start), 2, &mut lnks);
    }
    lnks.into_iter()
        .find(|p| {
            let stem = crate::leftovers::norm(&p.file_stem().unwrap_or_default().to_string_lossy());
            (stem == key || (key.len() >= 5 && stem.contains(&key))) && !stem.contains("uninstall") && !stem.contains("desinstal")
        })
        .and_then(|p| shell_png(&p.to_string_lossy()))
}

/// Last resort, present on every Windows: the Windows Installer icon for MSI products, else the generic app icon.
fn generic_icon(uninstall: &str) -> Option<Vec<u8>> {
    let sys = expand(r"%SystemRoot%\System32");
    let msi = uninstall.to_ascii_lowercase().contains("msiexec").then(|| png(&format!(r"{sys}\msiexec.exe"), 0)).flatten();
    msi.or_else(|| png(&format!(r"{sys}\shell32.dll"), 2))
}

/// Tries each source in turn until one yields an actual icon; the generic icon at the end means there's always one.
pub fn data_url(p: &Program) -> Option<String> {
    let icon = |c: Option<(String, i32)>| c.and_then(|(path, idx)| png(&path, idx));
    let unin = existing(&p.uninstall);
    let bytes = icon(existing(&p.display_icon))
        .or_else(|| icon(msi_icon(&p.uninstall)))
        .or_else(|| installer_cache_icon(&p.uninstall))
        .or_else(|| shortcut_icon(&p.name))
        .or_else(|| icon(find_exe(Path::new(&p.install_location), &p.name, 1)))
        // No install folder on record: look beside the uninstaller, but only trust a name match.
        .or_else(|| icon(unin.as_ref().and_then(|(u, _)| Path::new(u).parent().and_then(|d| find_exe(d, &p.name, 2)))))
        .or_else(|| icon(unin))
        .or_else(|| generic_icon(&p.uninstall))?;
    Some(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}

fn png(path: &str, idx: i32) -> Option<Vec<u8>> {
    unsafe {
        let mut icon = HICON::default();
        if ExtractIconExW(&HSTRING::from(path), idx, Some(&mut icon as *mut HICON), None, 1) == 0 || icon.is_invalid() {
            return None;
        }
        icon_png(icon)
    }
}

/// The icon Explorer would show for `path` (follows shortcuts to their icon).
fn shell_png(path: &str) -> Option<Vec<u8>> {
    unsafe {
        let mut fi = SHFILEINFOW::default();
        let r = SHGetFileInfoW(&HSTRING::from(path), FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut fi), size_of::<SHFILEINFOW>() as u32, SHGFI_ICON | SHGFI_LARGEICON);
        if r == 0 || fi.hIcon.is_invalid() {
            return None;
        }
        icon_png(fi.hIcon)
    }
}

/// Encodes an icon handle as PNG and destroys the handle.
unsafe fn icon_png(icon: HICON) -> Option<Vec<u8>> {
    {
        let mut info = ICONINFO::default();
        let rgba = GetIconInfo(icon, &mut info).ok().and_then(|_| pixels(&info));
        for b in [info.hbmColor, info.hbmMask] {
            if !b.is_invalid() {
                let _ = DeleteObject(b.into());
            }
        }
        let _ = DestroyIcon(icon);
        let (w, h, rgba) = rgba?;
        let mut buf = Vec::new();
        let mut e = png::Encoder::new(&mut buf, w, h);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header().ok()?.write_image_data(&rgba).ok()?;
        Some(buf)
    }
}

unsafe fn pixels(info: &ICONINFO) -> Option<(u32, u32, Vec<u8>)> {
    if info.hbmColor.is_invalid() {
        return None; // monochrome icon, not worth handling
    }
    let mut bm = BITMAP::default();
    GetObjectW(info.hbmColor.into(), size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut c_void));
    let (w, h) = (bm.bmWidth, bm.bmHeight);
    let hdc = GetDC(None);
    let grab = |bmp: HBITMAP| {
        let mut bi = BITMAPINFO::default();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let mut v = vec![0u8; (w * h * 4) as usize];
        let n = GetDIBits(hdc, bmp, 0, h as u32, Some(v.as_mut_ptr() as *mut c_void), &mut bi, DIB_RGB_COLORS);
        (n != 0).then_some(v)
    };
    let (color, mask) = (grab(info.hbmColor), grab(info.hbmMask));
    ReleaseDC(None, hdc);
    let mut px = color?;
    let has_alpha = px.chunks(4).any(|c| c[3] != 0);
    for (i, c) in px.chunks_mut(4).enumerate() {
        c.swap(0, 2); // BGRA -> RGBA
        if !has_alpha {
            // Old-style icon: transparency lives in the AND mask (white = transparent).
            c[3] = if mask.as_ref().is_some_and(|m| m[i * 4] != 0) { 0 } else { 255 };
        }
    }
    Some((w as u32, h as u32, px))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_msi_guids() {
        // Checked against this machine's registry: {9EDD8D53-...} (Gale) is installed under exactly this key name.
        assert_eq!(pack_guid("{9EDD8D53-3797-42C9-B0B0-61692D02EEB9}").as_deref(), Some("35D8DDE979739C240B0B1696D220EE9B"));
        assert_eq!(pack_guid("not-a-guid"), None);
    }

    #[test]
    fn folders_and_generic_tools_are_not_icon_sources() {
        assert!(existing(r"C:\Windows").is_none()); // a folder
        assert!(existing(r"C:\Windows\System32\msiexec.exe /X{1}").is_none()); // generic installer host
        assert!(existing(r#""C:\Windows\System32\notepad.exe",0"#).is_some());
    }

    #[test]
    fn every_program_gets_an_icon() {
        // No icon anywhere: not even a registered installer. Still must end up with the generic Windows icon.
        let mut p = Program {
            id: String::new(), name: "Zzz Nonexistent Program 9".into(), publisher: String::new(), version: String::new(),
            install_location: String::new(), uninstall: String::new(), size_kb: 0, install_date: String::new(),
            family: String::new(), display_icon: String::new(),
        };
        assert!(data_url(&p).is_some_and(|u| u.len() > 200), "generic app icon");
        p.uninstall = "MsiExec.exe /X{00000000-0000-0000-0000-000000000000}".into();
        assert!(data_url(&p).is_some_and(|u| u.len() > 200), "Windows Installer icon");
    }

    #[test]
    fn extracts_real_icon() {
        let p = Program {
            id: String::new(), name: String::new(), publisher: String::new(), version: String::new(),
            install_location: String::new(), uninstall: String::new(), size_kb: 0, install_date: String::new(),
            family: String::new(),
            display_icon: r#""%SystemRoot%\System32\notepad.exe",0"#.into(),
        };
        assert!(data_url(&p).is_some_and(|u| u.len() > 200));
    }
}

#[cfg(test)]
mod diag {
    use super::*;

    #[test]
    #[ignore] // diagnostic: cargo test icons_missing -- --ignored --nocapture
    fn icons_missing() {
        let all = crate::programs::list();
        let missing: Vec<_> = all.iter().filter(|p| data_url(p).is_none()).collect();
        println!("sin icono: {} / {}", missing.len(), all.len());
        for p in missing {
            println!("- {} | icon={:?} | unin={:?} | loc={:?}", p.name, p.display_icon, p.uninstall, p.install_location);
        }
        // How many fell all the way through to the generic icon?
        let enc = |b: Option<Vec<u8>>| b.map(|b| STANDARD.encode(b));
        let (g_msi, g_app) = (enc(generic_icon("msiexec")), enc(generic_icon("")));
        let generic: Vec<_> = all
            .iter()
            .filter(|p| data_url(p).map(|u| u.trim_start_matches("data:image/png;base64,").to_string()).is_some_and(|u| Some(&u) == g_msi.as_ref() || Some(&u) == g_app.as_ref()))
            .collect();
        println!("icono genérico: {} / {}", generic.len(), all.len());
        for p in generic {
            println!("  ~ {}", p.name);
        }
        let store = crate::store::list();
        let no_logo: Vec<_> = store.iter().filter(|p| p.display_icon.is_empty()).collect();
        println!("tienda sin icono: {} / {}", no_logo.len(), store.len());
        for p in no_logo {
            println!("- [store] {} | loc={:?}", p.name, p.install_location);
        }
    }
}
