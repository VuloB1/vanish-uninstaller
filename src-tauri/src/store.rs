use crate::programs::Program;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::{fs, os::windows::process::CommandExt, path::Path, process::Command};

#[derive(Deserialize)]
struct Pkg {
    name: String,
    full: String,
    family: String,
    version: String,
    publisher: String,
    loc: Option<String>,
}

// Names come from the Start menu (they're already localized); only apps with a Start entry are user-facing.
const SCRIPT: &str = r#"
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false
$s = @{}
Get-StartApps | ForEach-Object { if ($_.AppID -like '*!*') { $s[$_.AppID.Split('!')[0]] = $_.Name } }
$r = @(Get-AppxPackage | Where-Object { -not $_.IsFramework -and -not $_.NonRemovable -and $_.SignatureKind -ne 'System' -and $s.ContainsKey($_.PackageFamilyName) } | ForEach-Object {
  [pscustomobject]@{ name = $s[$_.PackageFamilyName]; full = $_.PackageFullName; family = $_.PackageFamilyName; version = $_.Version.ToString(); publisher = ($_.Publisher -replace '^.*?CN=([^,]+).*$', '$1'); loc = $_.InstallLocation }
})
ConvertTo-Json -InputObject $r -Compress
"#;

fn ps(script: &str) -> std::io::Result<std::process::Output> {
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
}

pub fn list() -> Vec<Program> {
    match ps(SCRIPT) {
        Ok(o) => parse(&String::from_utf8_lossy(&o.stdout)),
        Err(_) => vec![],
    }
}

fn parse(json: &str) -> Vec<Program> {
    let pkgs: Vec<Pkg> = serde_json::from_str(json.trim_start_matches('\u{feff}')).unwrap_or_default();
    let mut out: Vec<Program> = pkgs
        .into_iter()
        .map(|p| {
            let loc = p.loc.unwrap_or_default();
            Program {
                id: format!(r"APPX\{}", p.full),
                display_icon: logo(&loc).unwrap_or_default(),
                uninstall: format!("Remove-AppxPackage {}", p.full), // marker only; removal is dispatched on the id
                install_location: loc,
                name: p.name,
                // Some publishers are only a GUID (CN=xxxxxxxx-xxxx-...): useless to show.
                publisher: if p.publisher.len() == 36 && p.publisher.matches('-').count() == 4 { String::new() } else { p.publisher },
                version: p.version,
                family: p.family,
                size_kb: 0,
                install_date: String::new(),
            }
        })
        .collect();
    out.sort_by_key(|p| p.name.to_lowercase());
    out
}

pub fn remove(full: &str) -> Result<i32, String> {
    // The name goes into a PowerShell command line, so only the characters package names use are allowed.
    if full.is_empty() || !full.chars().all(|c| c.is_ascii_alphanumeric() || "._~-".contains(c)) {
        return Err("nombre de paquete inválido".into());
    }
    let o = ps(&format!("Remove-AppxPackage -Package '{full}' -ErrorAction Stop")).map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(0)
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().chars().take(300).collect())
    }
}

/// The app's small logo from its manifest, as a data URL. Picks the ~48px variant when there are several.
fn logo(loc: &str) -> Option<String> {
    let manifest = fs::read_to_string(Path::new(loc).join("AppxManifest.xml")).ok()?;
    let rel = ["Square44x44Logo=\"", "Square150x150Logo=\""].iter().find_map(|k| {
        let s = manifest.find(k)? + k.len();
        Some(manifest[s..s + manifest[s..].find('"')?].replace('\\', "/"))
    })?;
    let rel = Path::new(&rel);
    let stem = rel.file_stem()?.to_string_lossy().to_lowercase();
    let root = Path::new(loc);
    // The folder the manifest names first; if the files aren't there, anywhere in the package (resource packs
    // put them under e.g. Assets\<name>\en-US\, which is how iCloud ships).
    let mut found = Vec::new();
    gather(&root.join(rel.parent()?), 0, &stem, &mut found);
    if found.is_empty() {
        gather(root, 4, &stem, &mut found);
    }
    let best = found.into_iter().filter_map(|p| Some((rank(&p.file_name()?.to_string_lossy())?, p))).min_by_key(|(r, _)| *r)?.1;
    Some(format!("data:image/png;base64,{}", STANDARD.encode(fs::read(best).ok()?)))
}

fn gather(dir: &Path, depth: u8, stem: &str, out: &mut Vec<std::path::PathBuf>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let (p, n) = (e.path(), e.file_name().to_string_lossy().to_lowercase());
        if p.is_dir() && depth > 0 && out.len() < 200 {
            gather(&p, depth - 1, stem, out);
        } else if n.starts_with(stem) && n.ends_with(".png") {
            out.push(p);
        }
    }
}

/// Lower is better: the plain ~48px variants first, then the "unplated" ones (no backing tile), then anything else.
/// High-contrast and light-background variants are skipped or last.
fn rank(name: &str) -> Option<u8> {
    let n = name.to_ascii_lowercase();
    if n.contains("contrast") {
        return None;
    }
    let (alt, unplated) = (n.contains("altform"), n.contains("unplated") && !n.contains("lightunplated"));
    Some(if !alt && n.contains("targetsize-48") { 0 }
        else if !alt && n.contains("scale-200") { 1 }
        else if !alt && n.contains("scale-100") { 2 }
        else if unplated && n.contains("targetsize-48") { 3 }
        else if !alt { 4 }
        else if unplated { 5 }
        else { 6 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_powershell_json() {
        let j = "\u{feff}[{\"name\":\"Calculadora\",\"full\":\"Microsoft.Calc_1.0_x64__abc\",\"family\":\"Microsoft.Calc_abc\",\"version\":\"1.0\",\"publisher\":\"Microsoft\",\"loc\":null}]";
        let p = parse(j);
        assert_eq!((p[0].id.as_str(), p[0].family.as_str()), (r"APPX\Microsoft.Calc_1.0_x64__abc", "Microsoft.Calc_abc"));
        assert!(parse("garbage").is_empty());
    }

    #[test]
    fn rejects_unsafe_package_names() {
        assert!(remove("x'; Remove-Item C:\\ -Recurse; '").is_err());
        assert!(remove("").is_err());
    }

    #[test]
    fn finds_logo_outside_the_manifest_folder() {
        // iCloud: the manifest says `iCloud\Square44x44Logo.png` but the files live in Assets\iCloud\en-US\.
        let d = std::env::temp_dir().join("vanish_logo_test2");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join(r"Assets\iCloud\en-US")).unwrap();
        fs::write(d.join("AppxManifest.xml"), r#"<Package><Application Square150x150Logo="iCloud\Square150x150Logo.png" Square44x44Logo="iCloud\Square44x44Logo.png"/></Package>"#).unwrap();
        fs::write(d.join(r"Assets\iCloud\en-US\Square44x44Logo.altform-unplated_targetsize-48.png"), b"u").unwrap();
        fs::write(d.join(r"Assets\iCloud\en-US\Square44x44Logo.altform-lightunplated_targetsize-48.png"), b"l").unwrap();
        let url = logo(d.to_str().unwrap()).unwrap();
        assert_eq!(url, format!("data:image/png;base64,{}", STANDARD.encode(b"u"))); // unplated beats light-unplated
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn finds_logo_in_manifest() {
        let d = std::env::temp_dir().join("vanish_logo_test");
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("Assets")).unwrap();
        fs::write(d.join("AppxManifest.xml"), r#"<Package><Applications><Application><uap:VisualElements Square44x44Logo="Assets\Logo.png"/></Application></Applications></Package>"#).unwrap();
        fs::write(d.join(r"Assets\Logo.scale-100.png"), b"a").unwrap();
        fs::write(d.join(r"Assets\Logo.targetsize-48.png"), b"b").unwrap();
        fs::write(d.join(r"Assets\Logo.targetsize-48_altform-unplated.png"), b"c").unwrap();
        let url = logo(d.to_str().unwrap()).unwrap();
        assert_eq!(url, format!("data:image/png;base64,{}", STANDARD.encode(b"b")));
        let _ = fs::remove_dir_all(&d);
    }
}
