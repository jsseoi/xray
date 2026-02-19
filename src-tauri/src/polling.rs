use std::thread;
use std::time::Duration;
use std::sync::atomic::Ordering;
use tauri::{Manager, Emitter, PhysicalPosition, PhysicalSize, AppHandle, Monitor};
use crate::{accessibility, AppState};
use crate::constants::{EVENT_CAPTURE_CLICK, EVENT_ELEMENT_HOVER, POLLING_INTERVAL_MS, WINDOW_HIDE_DELAY_MS, WINDOW_LABEL_MAIN};

/// Spawns the background thread that handles mouse polling and screen capture logic.
pub fn spawn_polling_thread(handle: AppHandle) {
    thread::spawn(move || {
        let mut current_monitor_pos: Option<(i32, i32)> = None;
        let mut was_mouse_down = false;

        loop {
            // Sleep to maintain approx. 60 FPS polling rate
            thread::sleep(Duration::from_millis(POLLING_INTERVAL_MS));

            let state = handle.state::<AppState>();
            
            // If capture mode is not active, reset mouse state and continue.
            if !state.is_snip_active.load(Ordering::Relaxed) {
                was_mouse_down = false;
                continue;
            }

            let is_mouse_down = accessibility::is_mouse_left_down();

            // Detect Mouse Click (Trigger Capture)
            if is_mouse_down && !was_mouse_down {
                handle_click_capture(&handle, &state);
            }
            was_mouse_down = is_mouse_down;

            // If capture was triggered and mode ended, skip hover processing.
            if !state.is_snip_active.load(Ordering::Relaxed) {
                continue;
            }

            // Process Hover Logic (Scan UI elements and move overlay)
            process_hover_logic(&handle, &state, &mut current_monitor_pos);
        }
    });
}

/// Handles the logic when the user clicks to capture the screen.
fn handle_click_capture(handle: &AppHandle, state: &tauri::State<AppState>) {
    // 1. Disable capture mode
    state.is_snip_active.store(false, Ordering::Relaxed);
    
    // 2. Hide the overlay window
    if let Some(win) = handle.get_webview_window(WINDOW_LABEL_MAIN) {
        let _ = win.hide();
    }

    // 3. Wait for the window to disappear animation to finish
    thread::sleep(Duration::from_millis(WINDOW_HIDE_DELAY_MS));

    // 4. Retrieve the last hovered element info safely
    let rect_to_capture = state.current_info.lock()
        .map(|lock| lock.clone())
        .ok()
        .flatten();

    // 5. Emit capture-click event to frontend with element info.
    //    The frontend will show the save dialog and invoke capture commands.
    if let Some(info) = rect_to_capture {
        let _ = handle.emit(EVENT_CAPTURE_CLICK, info);
    }
}

/// Scans the UI element under the mouse and updates the overlay window position.
fn process_hover_logic(
    handle: &AppHandle, 
    state: &tauri::State<AppState>, 
    current_monitor_pos: &mut Option<(i32, i32)>
) {
    if let Some(mut info) = accessibility::get_element_at_mouse() {
        
        // Find which monitor the element is on and move the overlay window there
        if let Ok(monitors) = handle.available_monitors() {
            if let Some(target_monitor) = find_monitor_for_element(&monitors, &info) {
                update_overlay_window(handle, &target_monitor, current_monitor_pos, &mut info);
            }
        }

        // Update shared state
        if let Ok(mut lock) = state.current_info.lock() {
            *lock = Some(info.clone());
        }
        
        // Notify frontend
        let _ = handle.emit(EVENT_ELEMENT_HOVER, info);
    }
}

/// Finds the monitor that contains the given UI element.
fn find_monitor_for_element(monitors: &[Monitor], info: &accessibility::UIElementInfo) -> Option<Monitor> {
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
            return Some(m.clone());
        }
    }
    None
}

/// Moves the overlay window to the target monitor and adjusts coordinates.
fn update_overlay_window(
    handle: &AppHandle, 
    monitor: &Monitor, 
    current_monitor_pos: &mut Option<(i32, i32)>,
    info: &mut accessibility::UIElementInfo
) {
    if let Some(win) = handle.get_webview_window(WINDOW_LABEL_MAIN) {
        let m_pos = monitor.position();
        
        // Only move the window if the monitor has changed
        if *current_monitor_pos != Some((m_pos.x, m_pos.y)) {
            let _ = win.set_position(PhysicalPosition::new(m_pos.x, m_pos.y));
            let _ = win.set_size(PhysicalSize::new(monitor.size().width, monitor.size().height));
            *current_monitor_pos = Some((m_pos.x, m_pos.y));
        }
        
        // Transform global coordinates to window-local coordinates for the frontend
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