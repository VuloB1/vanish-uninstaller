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
    spawn_blocking(|| programs::list().iter().filter_map(|p| icon::data_url(p).map(|u| (p.id.clone(), u))).collect())
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

fn main() {
    std::thread::spawn(leftovers::purge_old);
    tauri::Builder::default()
        .manage(Scan::default())
        // Closing the only window ends the process outright: a background shell/icon call that hangs during a normal
        // exit would otherwise leave vanish.exe alive, and the installer then reports the app as still running.
        .on_window_event(|_, e| {
            if matches!(e, tauri::WindowEvent::Destroyed) {
                use windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
                unsafe { let _ = TerminateProcess(GetCurrentProcess(), 0); }
            }
        })
        .invoke_handler(tauri::generate_handler![is_admin, list_programs, store_programs, icons, uninstall, kill_processes, open_folder, scan, scan_junk, clean_junk, scan_registry, delete_leftovers])
        .run(tauri::generate_context!())
        .expect("error running app");
}
