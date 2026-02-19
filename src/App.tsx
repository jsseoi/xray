import { useEffect, useState } from "react";
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

  // Listen for element-hover events from the Rust backend
  useEffect(() => {
    const unlistenPromise = listen<UIElementInfo>("element-hover", (event) => {
      setHighlight(event.payload);
    });
    return () => { unlistenPromise.then((u) => u()); };
  }, []);

  // Listen for capture-click: show save dialog, then invoke capture command
  useEffect(() => {
    const unlistenPromise = listen<UIElementInfo>("capture-click", async (event) => {
      const info = event.payload;

      const path = await save({
        defaultPath: `capture-${Date.now()}.png`,
        filters: [{ name: "PNG Image", extensions: ["png"] }],
      });

      // Hide the overlay after the dialog closes (dialog may have brought the window back into view)
      await invoke("hide_window");

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
    </div>
  );
}

export default App;
