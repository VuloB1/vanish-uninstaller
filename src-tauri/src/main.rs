#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod icon;
mod junk;
mod leftovers;
mod programs;
mod regclean;
mod store;

use leftovers::{Leftover, Report, Scan};
use programs::Program;
use std::os::windows::process::CommandExt;
use tauri::{async_runtime::spawn_blocking, State};

#[tauri::command]
async fn list_programs() -> Vec<Program> {
    spawn_blocking(programs::list).await.unwrap_or_default()
}

/// Store apps are fetched separately: PowerShell takes a second or two and shouldn't delay the main list.
#[tauri::command]
async fn store_programs() -> Vec<Program> {
    spawn_blocking(store::list).await.unwrap_or_default()
}

/// Icons as data URLs keyed by program id; loaded after the list so the list shows instantly.
#[tauri::command]
async fn icons() -> std::collections::HashMap<String, String> {
    spawn_blocking(|| {
        use std::sync::{atomic::{AtomicUsize, Ordering}, Mutex};
        let list = programs::list();
        let (next, out) = (AtomicUsize::new(0), Mutex::new(std::collections::HashMap::new()));
        // Each icon is an independent shell/GDI call, so a few workers pulling from one counter cut the wait several times over.
        std::thread::scope(|s| {
            for _ in 0..8.min(list.len().max(1)) {
                s.spawn(|| {
                    while let Some(p) = list.get(next.fetch_add(1, Ordering::Relaxed)) {
                        if let Some(u) = icon::data_url(p) {
                            out.lock().unwrap().insert(p.id.clone(), u);
                        }
                    }
                });
            }
        });
        out.into_inner().unwrap()
    })
    .await
    .unwrap_or_default()
}

/// Runs the program's own uninstaller and waits for it to finish.
#[tauri::command]
async fn uninstall(id: String) -> Result<i32, String> {
    if let Some(full) = id.strip_prefix(r"APPX\") {
        let full = full.to_string();
        return spawn_blocking(move || store::remove(&full)).await.map_err(|e| e.to_string())?;
    }
    spawn_blocking(move || {
        let p = programs::list().into_iter().find(|p| p.id == id).ok_or("programa no encontrado")?;
        std::process::Command::new("cmd")
            .raw_arg("/C")
            .raw_arg(format!("\"{}\"", p.uninstall))
            .status()
            .map(|s| s.code().unwrap_or(-1))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Forced removal, step one: close the program's processes. The path comes from the registry, not from the UI.
#[tauri::command]
async fn kill_processes(id: String) -> Result<(), String> {
    spawn_blocking(move || {
        let p = programs::list().into_iter().find(|p| p.id == id).ok_or("programa no encontrado")?;
        leftovers::kill_in(&p.install_location)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Opens the program's install folder in Explorer. The path comes from the registry entry, not from the UI.
#[tauri::command]
async fn open_folder(id: String) -> Result<(), String> {
    spawn_blocking(move || {
        let p = programs::list().into_iter().find(|p| p.id == id).ok_or("programa no encontrado")?;
        let dir = std::path::Path::new(&p.install_location);
        if p.install_location.is_empty() || !dir.is_dir() {
            return Err("la carpeta de instalación no existe".to_string());
        }
        std::process::Command::new("explorer").arg(dir).spawn().map(|_| ()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The window is created hidden (no white flash while the webview loads); the UI calls this when it has painted.
#[tauri::command]
fn app_ready(window: tauri::WebviewWindow) {
    let _ = window.show();
}

/// Opens the author's GitHub pages in the browser. Only this account's URLs are allowed.
#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/VuloB1") {
        return Err("enlace no permitido".to_string());
    }
    std::process::Command::new("explorer").arg(url).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Leftovers for a folder or file the user dropped or picked. Results go to the same state as a normal scan.
#[tauri::command]
async fn scan_path(paths: Vec<String>, state: State<'_, Scan>) -> Result<Vec<Leftover>, String> {
    let found = spawn_blocking(move || leftovers::scan_paths(&paths, &programs::list())).await.map_err(|e| e.to_string())??;
    *state.0.lock().unwrap() = found.iter().map(|l| (l.id.clone(), l.clone())).collect();
    Ok(found)
}

#[tauri::command]
async fn kill_path(path: String) -> Result<(), String> {
    spawn_blocking(move || leftovers::kill_path(&path)).await.map_err(|e| e.to_string())?
}

/// Paths given on the command line (e.g. a folder dropped on the exe's icon), which works even though the app runs elevated.
#[tauri::command]
fn startup_paths() -> Vec<String> {
    std::env::args().skip(1).filter(|a| !a.starts_with('-') && std::path::Path::new(a).exists()).collect()
}

#[tauri::command]
async fn scan(programs: Vec<Program>, state: State<'_, Scan>) -> Result<Vec<Leftover>, String> {
    let found = spawn_blocking(move || leftovers::scan(&programs, &programs::list())).await.map_err(|e| e.to_string())?;
    *state.0.lock().unwrap() = found.iter().map(|l| (l.id.clone(), l.clone())).collect();
    Ok(found)
}

#[tauri::command]
async fn scan_junk() -> Vec<junk::JunkCat> {
    spawn_blocking(junk::scan).await.unwrap_or_default()
}

#[tauri::command]
async fn clean_junk(ids: Vec<String>) -> Result<junk::JunkReport, String> {
    spawn_blocking(move || junk::clean(&ids)).await.map_err(|e| e.to_string())
}

/// Registry problems share the leftover state and the delete path (backup first, then remove).
#[tauri::command]
async fn scan_registry(state: State<'_, Scan>) -> Result<Vec<Leftover>, String> {
    let found = spawn_blocking(regclean::scan).await.map_err(|e| e.to_string())?;
    *state.0.lock().unwrap() = found.iter().map(|l| (l.id.clone(), l.clone())).collect();
    Ok(found)
}

#[tauri::command]
async fn delete_leftovers(ids: Vec<String>, state: State<'_, Scan>) -> Result<Report, String> {
    Ok(leftovers::delete(&state, ids))
}

#[tauri::command]
fn is_admin() -> bool {
    use windows::Win32::{Foundation::*, Security::*, System::Threading::*};
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut e = TOKEN_ELEVATION::default();
        let mut n = 0u32;
        let size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        let ok = GetTokenInformation(token, TokenElevation, Some(&mut e as *mut _ as *mut _), size, &mut n).is_ok();
        let _ = CloseHandle(token);
        ok && e.TokenIsElevated != 0
    }
}

/// Ends this process and the webview's helper processes under it, so nothing of the app is left running.
/// Only `msedgewebview2.exe` children are touched: anything else we started (a program's uninstaller) keeps going.
fn exit_all() {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::{
            Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS},
            Threading::{GetCurrentProcess, GetCurrentProcessId, OpenProcess, TerminateProcess, PROCESS_TERMINATE},
        },
    };
    unsafe {
        let mut all = Vec::new(); // (pid, parent pid, is webview)
        if let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            let mut e = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            let mut ok = Process32FirstW(snap, &mut e).is_ok();
            while ok {
                let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                let web = String::from_utf16_lossy(&e.szExeFile[..len]).eq_ignore_ascii_case("msedgewebview2.exe");
                all.push((e.th32ProcessID, e.th32ParentProcessID, web));
                ok = Process32NextW(snap, &mut e).is_ok();
            }
            let _ = CloseHandle(snap);
        }
        let mut tree = vec![GetCurrentProcessId()];
        let mut i = 0;
        while i < tree.len() {
            let parent = tree[i];
            let kids: Vec<u32> = all.iter().filter(|(_, ppid, web)| *web && *ppid == parent).map(|(pid, ..)| *pid).collect();
            for k in kids {
                if !tree.contains(&k) {
                    tree.push(k);
                }
            }
            i += 1;
        }
        for pid in tree.into_iter().skip(1) {
            if let Ok(h) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                let _ = TerminateProcess(h, 0);
                let _ = CloseHandle(h);
            }
        }
        let _ = TerminateProcess(GetCurrentProcess(), 0);
    }
}

fn main() {
    std::thread::spawn(leftovers::purge_old);
    tauri::Builder::default()
        // A second launch (e.g. the installer reopening the app, or a path dropped on the exe) hands over to the running window.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            use tauri::{Emitter, Manager};
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
            let paths: Vec<String> = args.into_iter().skip(1).filter(|a| !a.starts_with('-') && std::path::Path::new(a).exists()).collect();
            if !paths.is_empty() {
                let _ = app.emit("open-paths", paths);
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(Scan::default())
        .setup(|app| {
            use tauri::Manager;
            // Failsafe: if the UI never reports in, don't leave the user with an invisible app.
            if let Some(w) = app.get_webview_window("main") {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    let _ = w.show();
                });
            }
            Ok(())
        })
        // Closing the only window ends the process outright: a background shell/icon call that hangs during a normal
        // exit would otherwise leave vanish.exe alive, and the installer then reports the app as still running.
        .on_window_event(|_, e| {
            if matches!(e, tauri::WindowEvent::Destroyed) {
                exit_all();
            }
        })
        .invoke_handler(tauri::generate_handler![is_admin, list_programs, store_programs, icons, uninstall, kill_processes, open_folder, open_link, app_ready, scan_path, kill_path, startup_paths, scan, scan_junk, clean_junk, scan_registry, delete_leftovers])
        .run(tauri::generate_context!())
        .expect("error running app");
}
