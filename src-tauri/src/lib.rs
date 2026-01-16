mod accessibility;
mod capture;

use tauri::{
    menu::{Menu, MenuItem}, 
    tray::TrayIconBuilder, 
    Manager, Emitter, PhysicalPosition, PhysicalSize
};
use std::thread;
use std::time::Duration;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Manages the application's global state.
struct AppState {
    /// Information about the UI element currently being hovered over.
    current_info: Mutex<Option<accessibility::UIElementInfo>>,
    /// Whether capture mode (overlay enabled) is currently active.
    is_snip_active: AtomicBool,
}

/// Command to hide the main overlay window.
#[tauri::command]
fn hide_window(window: tauri::WebviewWindow, state: tauri::State<AppState>) {
    state.is_snip_active.store(false, Ordering::Relaxed);
    let _ = window.hide();
}

/// Starts capture mode (shows the window and enables accessibility scanning).
fn start_snip(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.is_snip_active.store(true, Ordering::Relaxed);
    }
    if let Some(window) = app.get_webview_window("main") {
        // Allows mouse clicks to pass through the overlay to the underlying application.
        let _ = window.set_ignore_cursor_events(true);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
                        start_snip(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Set up the system tray menu.
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let snip_i = MenuItem::with_id(app, "snip", "Snip Screen", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&snip_i, &quit_i])?;
            
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app: &tauri::AppHandle, event| {
                    match event.id.as_ref() {
                        "quit" => app.exit(0),
                        "snip" => start_snip(app),
                        _ => {}
                    }
                })
                .build(app)?;

            let handle = app.handle().clone();
            
            // 1. Accessibility and input polling thread.
            let handle_access = handle.clone();
            thread::spawn(move || {
                let mut current_monitor_pos: Option<(i32, i32)> = None;
                let mut was_mouse_down = false;

                loop {
                    let state = handle_access.state::<AppState>();
                    
                    if state.is_snip_active.load(Ordering::Relaxed) {
                         let is_mouse_down = accessibility::is_mouse_left_down();
                         
                         if is_mouse_down && !was_mouse_down {
                             state.is_snip_active.store(false, Ordering::Relaxed);
                             if let Some(win) = handle_access.get_webview_window("main") {
                                 let _ = win.hide();
                             }

                             thread::sleep(Duration::from_millis(150));

                             let mut rect_to_capture = None;
                             if let Ok(lock) = state.current_info.lock() {
                                 rect_to_capture = lock.clone();
                             }

                             if let Some(info) = rect_to_capture {
                                 if let Err(e) = capture::capture_rect(
                                     info.global_x, 
                                     info.global_y, 
                                     info.width, 
                                     info.height,
                                     info.window_id,
                                     info.role.clone()
                                 ) {
                                     eprintln!("Capture failed: {}", e);
                                 } else {
                                     println!("Copied to clipboard!");
                                 }
                             }
                         }
                         was_mouse_down = is_mouse_down;

                         if !state.is_snip_active.load(Ordering::Relaxed) {
                             thread::sleep(Duration::from_millis(16));
                             continue;
                         }

                         if let Some(mut info) = accessibility::get_element_at_mouse() {
                            if let Ok(monitors) = handle_access.available_monitors() {
                                let mut target_monitor = None;
                                for m in monitors {
                                    let pos = m.position();
                                    let size = m.size();
                                    let scale_factor = m.scale_factor();
                                    let logical_x = pos.x as f64 / scale_factor;
                                    let logical_y = pos.y as f64 / scale_factor;
                                    let logical_w = size.width as f64 / scale_factor;
                                    let logical_h = size.height as f64 / scale_factor;

                                    if info.global_x >= logical_x && info.global_x < (logical_x + logical_w) &&
                                       info.global_y >= logical_y && info.global_y < (logical_y + logical_h) {
                                        target_monitor = Some(m);
                                        break;
                                    }
                                }

                                if let Some(m) = target_monitor {
                                    if let Some(win) = handle_access.get_webview_window("main") {
                                        let m_pos = m.position();
                                        if current_monitor_pos != Some((m_pos.x, m_pos.y)) {
                                            let _ = win.set_position(PhysicalPosition::new(m_pos.x, m_pos.y));
                                            let _ = win.set_size(PhysicalSize::new(m.size().width, m.size().height));
                                            current_monitor_pos = Some((m_pos.x, m_pos.y));
                                        }
                                        
                                        if let Ok(win_pos) = win.outer_position() {
                                            if let Ok(scale_factor) = win.scale_factor() {
                                                let win_logical_x = win_pos.x as f64 / scale_factor;
                                                let win_logical_y = win_pos.y as f64 / scale_factor;
                                                
                                                info.x = info.global_x - win_logical_x;
                                                info.y = info.global_y - win_logical_y;
                                            }
                                        }
                                    }
                                }
                            }

                            if let Ok(mut lock) = state.current_info.lock() {
                                *lock = Some(info.clone());
                            }
                            
                            let _ = handle_access.emit("element-hover", info);
                        }
                    } else {
                        was_mouse_down = false;
                    }
                    thread::sleep(Duration::from_millis(16));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![capture::capture_rect, hide_window])
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application");
}
