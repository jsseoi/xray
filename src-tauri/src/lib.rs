mod accessibility;
mod capture;
mod constants;
mod polling;

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::constants::WINDOW_LABEL_MAIN;

const PREF_FILE: &str = "copy_to_clipboard";

/// Manages the application's global state.
pub struct AppState {
    /// Information about the UI element currently being hovered over.
    pub current_info: Mutex<Option<accessibility::UIElementInfo>>,
    /// Whether capture mode (overlay enabled) is currently active.
    pub is_snip_active: AtomicBool,
    /// Whether to also copy the capture to the clipboard.
    pub copy_to_clipboard: AtomicBool,
}

fn load_pref(app: &tauri::AppHandle) -> bool {
    app.path().app_config_dir()
        .ok()
        .and_then(|dir| std::fs::read_to_string(dir.join(PREF_FILE)).ok())
        .map(|s| s.trim() != "false")
        .unwrap_or(true)
}

fn save_pref(app: &tauri::AppHandle, value: bool) {
    if let Ok(dir) = app.path().app_config_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(PREF_FILE), value.to_string());
    }
}

/// Command to hide the main overlay window.
#[tauri::command]
fn hide_window(window: tauri::WebviewWindow, state: tauri::State<AppState>) {
    state.is_snip_active.store(false, Ordering::Relaxed);
    let _ = window.hide();
}

/// Starts capture mode: shows the overlay window and enables accessibility scanning.
fn start_capture_session(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.is_snip_active.store(true, Ordering::Relaxed);
    }
    if let Some(window) = app.get_webview_window(WINDOW_LABEL_MAIN) {
        let _ = window.set_ignore_cursor_events(true);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(debug_assertions)]
    let builder = builder.plugin(tauri_plugin_log::Builder::new().build());

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut("CommandOrControl+Shift+X")
                .expect("Failed to register capture shortcut")
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        start_capture_session(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let copy_enabled = load_pref(app.handle());

            app.manage(AppState {
                current_info: Mutex::new(None),
                is_snip_active: AtomicBool::new(false),
                copy_to_clipboard: AtomicBool::new(copy_enabled),
            });

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let snip_i = MenuItem::with_id(app, "snip", "Snip Screen", true, None::<&str>)?;
            let copy_i = CheckMenuItem::with_id(app, "copy_to_clipboard", "Copy to Clipboard", true, copy_enabled, None::<&str>)?;
            let menu = Menu::with_items(app, &[&snip_i, &copy_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app: &tauri::AppHandle, event| {
                    match event.id.as_ref() {
                        "quit" => app.exit(0),
                        "snip" => start_capture_session(app),
                        "copy_to_clipboard" => {
                            if let Some(state) = app.try_state::<AppState>() {
                                let new_val = !state.copy_to_clipboard.load(Ordering::Relaxed);
                                state.copy_to_clipboard.store(new_val, Ordering::Relaxed);
                                save_pref(app, new_val);
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            polling::spawn_polling_thread(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            capture::capture_rect_to_file,
            hide_window
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application");
}
