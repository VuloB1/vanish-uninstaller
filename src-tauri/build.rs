fn main() {
    let mut attrs = tauri_build::Attributes::new();
    // Release only: an admin manifest would stop `tauri dev` from launching out of a normal terminal.
    if std::env::var("PROFILE").is_ok_and(|p| p == "release") {
        attrs = attrs.windows_attributes(tauri_build::WindowsAttributes::new().app_manifest(include_str!("admin.manifest")));
    }
    tauri_build::try_build(attrs).expect("tauri build failed")
}
