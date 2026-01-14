# xray

[한국어](./README.ko.md)

<div align="center">
  <img 
    src="https://github.com/user-attachments/assets/e15d9ae4-d06d-4130-9564-3e446bc4b6db" 
    alt="xray-apple"
    width="80%"
    height="80%"
  />
  <p><em>Web page element capture</em></p>
</div>



Universal UI capture and inspection tool.

macOS Desktop application for inspecting and capturing UI elements across the entire system, similar to Chrome DevTools inspector but for the OS.

## Features

- **Global Inspector:** Hover over any window, button, or UI element on your screen to see it highlighted.
- **Smart Capture:** Click the highlighted element to instantly capture it to your clipboard.
- **System Tray Integration:** Runs quietly in the background.
- **Global Shortcut:** Activate the inspector on demand.

## Setup & Installation

1.  **Clone the repository**
2.  **Install Dependencies:**
    ```bash
    npm install
    ```
3.  **Run Development Build:**
    ```bash
    npm run tauri dev
    ```

## How to Use

1.  **Grant Permissions:**
    *   On first launch, you must grant **Accessibility** and **Screen Recording** permissions to the app (or your terminal if running in dev mode).
    *   If the app doesn't work, check *System Settings > Privacy & Security* and ensure permissions are enabled.

2.  **Start Inspection:**
    *   The app launches in the background (check the Menu Bar for the icon).
    *   Press **`Cmd + Shift + X`** (or `Ctrl + Shift + X`) to activate the overlay.

3.  **Capture:**
    *   Move your mouse to highlight the desired UI element.
    *   **Click** to capture.
    *   The overlay will close, and the screenshot is now in your **Clipboard**. Paste it anywhere (`Cmd + V`).

4.  **Quit:**
    *   Click the tray icon in the menu bar and select **Quit**.

## Architecture

- **Frontend:** React + TypeScript (Visual Overlay)
- **Backend:** Rust (Tauri, Accessibility API, CoreGraphics)
- **State Management:** Tauri Events (`element-hover`)

## License

[MIT](LICENSE)
