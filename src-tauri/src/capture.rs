use std::process::Command;

/// Captures a specific rectangular region of the screen or a window.
///
/// If `window_id` is provided and valid, it captures that specific window.
/// Otherwise, it captures the screen region defined by `x`, `y`, `width`, `height`.
#[tauri::command]
pub fn capture_rect(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    window_id: u32,
    role: String,
) -> Result<(), String> {
    let mut command = Command::new("screencapture");
    command.arg("-c"); // Copy to clipboard

    // Decide whether to capture the window itself or a specific rectangular area.
    // Capturing by Window ID (-l) is cleaner for rounded corners and shadows.
    if role.contains("Window") && window_id > 0 {
        command.arg("-l");
        command.arg(window_id.to_string());
    } else {
        let region = format!("{},{},{},{}", x, y, width, height);
        command.arg("-R");
        command.arg(&region);
    }

    let output = command.output().map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(())
}
