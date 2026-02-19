mod accessibility;
mod capture;
mod constants;
mod polling;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::constants::{EVENT_SHOW_SETTINGS, WINDOW_LABEL_MAIN};

/// Manages the application's global state.
pub struct AppState {
    /// Information about the UI element currently being hovered over.
    pub current_info: Mutex<Option<accessibility::UIElementInfo>>,
    /// Whether capture mode (overlay enabled) is currently active.
    pub is_snip_active: AtomicBool,
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
        // Allows mouse clicks to pass through the overlay to the underlying application.
        // Note: We still detect clicks via global hooks in polling.rs.
        let _ = window.set_ignore_cursor_events(true);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // Only enable logging in debug builds. 
    // In release builds, log macros (info!, error!, etc.) will do nothing.
    #[cfg(debug_assertions)]
    let builder = builder.plugin(tauri_plugin_log::Builder::new().build());

    builder
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            current_info: Mutex::new(None),
            is_snip_active: AtomicBool::new(false),
        })
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut("CommandOrControl+Shift+X")
                .expect("Failed to register global shortcut")
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        start_capture_session(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Set up the system tray menu.
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let snip_i = MenuItem::with_id(app, "snip", "Snip Screen", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&snip_i, &settings_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app: &tauri::AppHandle, event| {
                    match event.id.as_ref() {
                        "quit" => app.exit(0),
                        "snip" => start_capture_session(app),
                        "settings" => {
                            if let Some(win) = app.get_webview_window(WINDOW_LABEL_MAIN) {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                            let _ = app.emit(EVENT_SHOW_SETTINGS, ());
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Start the background polling thread for accessibility and input handling.
            polling::spawn_polling_thread(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            capture::capture_rect,
            capture::capture_rect_to_file,
            hide_window
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application");
}
