import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface UIElementInfo {
  x: number;
  y: number;
  width: number;
  height: number;
}

function App() {
  const [highlight, setHighlight] = useState<UIElementInfo | null>(null);

  useEffect(() => {
    // Rust 백엔드로부터 'element-hover' 이벤트를 수신하여 하이라이트 박스 좌표를 갱신합니다.
    const unlistenPromise = listen<UIElementInfo>("element-hover", (event) => {
      setHighlight(event.payload);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    // ESC 키를 누르면 오버레이 창을 숨깁니다.
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        invoke("hide_window");
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

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
            border: "2px solid red",
            backgroundColor: "rgba(255, 0, 0, 0.1)",
            pointerEvents: "none",
            boxSizing: "border-box",
            transition: "all 0.05s ease-out", // 부드러운 박스 이동
          }}
        />
      )}
    </div>
  );
}

export default App;
