use std::process::Command;

/// Captures a specific rectangular region or window and saves it to a file.
///
/// `path` is the full file path chosen by the user via the save dialog.
/// If `copy_to_clipboard` in AppState is true, the saved file is also copied
/// to the clipboard via osascript (no extra shutter sound).
#[tauri::command]
pub fn capture_rect_to_file(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    window_id: u32,
    role: String,
    path: String,
    state: tauri::State<crate::AppState>,
) -> Result<(), String> {
    let copy_to_clipboard = state.copy_to_clipboard.load(std::sync::atomic::Ordering::Relaxed);
    let mut command = Command::new("screencapture");

    if role.contains("Window") && window_id > 0 {
        command.arg("-l");
        command.arg(window_id.to_string());
    } else {
        let region = format!("{},{},{},{}", x, y, width, height);
        command.arg("-R");
        command.arg(&region);
    }

    command.arg(&path);

    let output = command.output().map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    if copy_to_clipboard {
        let script = format!(
            "set the clipboard to (read (POSIX file \"{}\") as «class PNGf»)",
            path
        );
        let _ = Command::new("osascript").arg("-e").arg(&script).output();
    }

    Ok(())
}
