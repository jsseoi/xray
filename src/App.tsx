import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import "./App.css";

interface UIElementInfo {
  x: number;
  y: number;
  width: number;
  height: number;
  role: string;
  globalX: number;
  globalY: number;
  windowId: number;
}

function App() {
  const [highlight, setHighlight] = useState<UIElementInfo | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [copyToClipboard, setCopyToClipboard] = useState(
    () => localStorage.getItem("xray-copy-to-clipboard") !== "false"
  );

  // Keep copyToClipboard accessible inside the capture-click listener without re-registering
  const copyToClipboardRef = useRef(copyToClipboard);
  useEffect(() => {
    copyToClipboardRef.current = copyToClipboard;
  }, [copyToClipboard]);

  // Listen for element-hover events from the Rust backend
  useEffect(() => {
    const unlistenPromise = listen<UIElementInfo>("element-hover", (event) => {
      setHighlight(event.payload);
    });
    return () => { unlistenPromise.then((u) => u()); };
  }, []);

  // Listen for capture-click: show save dialog, then invoke capture commands
  useEffect(() => {
    const unlistenPromise = listen<UIElementInfo>("capture-click", async (event) => {
      const info = event.payload;

      const path = await save({
        defaultPath: `capture-${Date.now()}.png`,
        filters: [{ name: "PNG Image", extensions: ["png"] }],
      });

      if (!path) return; // User cancelled the dialog

      await invoke("capture_rect_to_file", {
        x: info.globalX,
        y: info.globalY,
        width: info.width,
        height: info.height,
        windowId: info.windowId,
        role: info.role,
        path,
      });

      if (copyToClipboardRef.current) {
        await invoke("capture_rect", {
          x: info.globalX,
          y: info.globalY,
          width: info.width,
          height: info.height,
          windowId: info.windowId,
          role: info.role,
        });
      }
    });
    return () => { unlistenPromise.then((u) => u()); };
  }, []);

  // Listen for show-settings events from the tray menu
  useEffect(() => {
    const unlistenPromise = listen("show-settings", () => {
      setShowSettings((prev) => !prev);
    });
    return () => { unlistenPromise.then((u) => u()); };
  }, []);

  // Hide the overlay window when ESC is pressed
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        invoke("hide_window");
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  // Helper to remove "AX" prefix from accessibility roles
  const formatRole = (role: string) => role.replace(/^AX/, "");

  return (
    <div
      style={{
        width: "100vw",
        height: "100vh",
        cursor: "crosshair",
        position: "relative",
      }}
    >
      {highlight && (
        <div
          style={{
            position: "absolute",
            left: `${highlight.x}px`,
            top: `${highlight.y}px`,
            width: `${highlight.width}px`,
            height: `${highlight.height}px`,
            // Use inset box-shadow instead of border to prevent clipping on screen edges
            boxShadow: "inset 0 0 0 2px red",
            backgroundColor: "rgba(255, 0, 0, 0.1)",
            pointerEvents: "none",
            boxSizing: "border-box",
            transition: "all 0.05s ease-out",
          }}
        >
          {/* Info HUD Label */}
          <div
            style={{
              position: "absolute",
              top: highlight.y < 30 ? "100%" : "-26px",
              left: "0",
              backgroundColor: "#cc0000",
              color: "white",
              padding: "2px 6px",
              fontSize: "11px",
              fontFamily: "system-ui, sans-serif",
              fontWeight: "bold",
              whiteSpace: "nowrap",
              borderRadius: "2px",
              zIndex: 10000,
              boxShadow: "0 1px 3px rgba(0,0,0,0.3)",
              marginTop: highlight.y < 30 ? "4px" : "0",
            }}
          >
            <span style={{ opacity: 0.9 }}>{formatRole(highlight.role)}</span>
            <span style={{ margin: "0 4px", opacity: 0.5 }}>|</span>
            <span style={{ fontFamily: "monospace" }}>
              {Math.round(highlight.width)} × {Math.round(highlight.height)}
            </span>
          </div>
        </div>
      )}

      {/* Settings Panel */}
      {showSettings && (
        <div
          style={{
            position: "fixed",
            bottom: "20px",
            right: "20px",
            width: "260px",
            backgroundColor: "#1a1a1a",
            border: "1px solid #333",
            borderRadius: "6px",
            boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
            fontFamily: "system-ui, sans-serif",
            fontSize: "12px",
            color: "white",
            zIndex: 20000,
            pointerEvents: "auto",
          }}
        >
          {/* Header */}
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              padding: "8px 12px",
              borderBottom: "1px solid #333",
              backgroundColor: "#cc0000",
              borderRadius: "5px 5px 0 0",
            }}
          >
            <span style={{ fontWeight: "bold" }}>xray Settings</span>
            <button
              onClick={() => setShowSettings(false)}
              style={{
                background: "none",
                border: "none",
                color: "white",
                cursor: "pointer",
                fontSize: "14px",
                lineHeight: 1,
                padding: "0",
              }}
            >
              ×
            </button>
          </div>

          {/* Body */}
          <div style={{ padding: "12px" }}>
            <div
              style={{
                marginBottom: "4px",
                opacity: 0.6,
                fontSize: "10px",
                textTransform: "uppercase",
                letterSpacing: "0.5px",
              }}
            >
              Capture
            </div>
            <label
              style={{
                display: "flex",
                alignItems: "flex-start",
                gap: "8px",
                cursor: "pointer",
                padding: "4px 0",
              }}
            >
              <input
                type="checkbox"
                checked={copyToClipboard}
                onChange={(e) => {
                  const value = e.target.checked;
                  setCopyToClipboard(value);
                  localStorage.setItem("xray-copy-to-clipboard", String(value));
                }}
                style={{ marginTop: "1px", accentColor: "#cc0000" }}
              />
              <span>
                Copy to clipboard
                <br />
                <span style={{ opacity: 0.5, fontSize: "10px" }}>
                  좌클릭 시 클립보드에도 저장
                </span>
              </span>
            </label>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
