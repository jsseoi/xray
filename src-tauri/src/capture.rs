use std::process::Command;

#[tauri::command]
pub fn capture_rect(x: f64, y: f64, width: f64, height: f64, window_id: u32, role: String) -> Result<(), String> {
    let mut command = Command::new("screencapture");
    command.arg("-c"); // Copy to clipboard

    // Decide whether to capture the window itself or a specific rectangular area.
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


/// Command to delete the captured file.
#[tauri::command]
pub fn delete_capture(path: String) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| e.to_string())
}
