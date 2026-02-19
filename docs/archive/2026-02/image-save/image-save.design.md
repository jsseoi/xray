# Design: image-save

## 1. Overview

**Feature Name**: image-save
**Plan Reference**: `docs/01-plan/features/image-save.plan.md`
**Date**: 2026-02-19
**Phase**: Design

## 2. 현재 코드베이스 분석

### 핵심 발견사항

| 항목 | 상태 | 비고 |
|------|------|------|
| `tauri-plugin-dialog` (Cargo) | ✅ 이미 존재 | `2.5.0` — 추가 불필요 |
| `@tauri-apps/plugin-dialog` (npm) | ❌ 없음 | 추가 필요 |
| 클릭 처리 위치 | Rust (`polling.rs`) | 프론트엔드 X |
| UIElementInfo | Rust 구조체 | `global_x/y`, `window_id` 포함 |
| 프론트엔드 UIElementInfo | `App.tsx` 인터페이스 | `x, y, width, height, role` 만 있음 |

### 클릭 처리 현재 흐름

```
polling.rs: handle_click_capture()
  1. is_snip_active = false
  2. window.hide()
  3. sleep(WINDOW_HIDE_DELAY_MS)
  4. capture::capture_rect(global_x, global_y, width, height, window_id, role)
     → screencapture -c  (클립보드)
```

## 3. 아키텍처 설계

### 변경된 흐름 (이벤트 기반)

```
polling.rs: handle_click_capture()  [수정]
  1. is_snip_active = false
  2. window.hide()
  3. sleep(WINDOW_HIDE_DELAY_MS)
  4. handle.emit("capture-click", CapturePayload)  ← 변경: capture 대신 이벤트 emit

App.tsx: listen("capture-click")  [신규]
  5. save() 다이얼로그 열기 (macOS NSSavePanel)
  6a. 취소 → 아무것도 안 함
  6b. 확인(path) →
      invoke("capture_rect_to_file", { ...payload, path })
      if (copyToClipboard) invoke("capture_rect", { ...payload })
```

### 왜 Rust 직접 다이얼로그가 아닌 프론트엔드 이벤트 방식인가?

- 설정 상태(`copyToClipboard`)는 프론트엔드(localStorage)에 존재
- 취소 시 아무것도 안 해야 함 — Rust에서 처리하면 복잡도 증가
- 기존 `element-hover` 이벤트 패턴과 일관성 유지
- 숨겨진 WebView에서도 JS 실행 및 네이티브 다이얼로그 호출 가능 (Tauri 2 지원)

## 4. 데이터 모델

### CapturePayload (새 이벤트 페이로드)

```rust
// polling.rs에서 emit할 구조체 (accessibility.rs의 UIElementInfo 재사용 가능)
// UIElementInfo는 이미 Serialize 구현됨
pub struct UIElementInfo {
    pub x: f64,        // 윈도우 로컬 좌표 (프론트엔드 표시용)
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub global_x: f64, // capture_rect에 전달할 글로벌 좌표
    pub global_y: f64,
    pub window_id: u32,
    pub role: String,
}
```

프론트엔드 인터페이스 확장:
```typescript
// App.tsx - 기존 인터페이스 확장
interface UIElementInfo {
  x: number;
  y: number;
  width: number;
  height: number;
  role: string;
  globalX: number;   // 추가 (camelCase — Tauri 자동 변환)
  globalY: number;   // 추가
  windowId: number;  // 추가
}
```

> Tauri는 Rust의 `snake_case`를 JS의 `camelCase`로 자동 변환한다.
> `global_x` → `globalX`, `window_id` → `windowId`

## 5. 컴포넌트 설계

### 5-1. Rust Backend

#### capture.rs — `capture_rect_to_file` 추가

```rust
/// 캡처 결과를 파일로 저장한다.
/// path: 사용자가 다이얼로그에서 선택한 저장 경로
#[tauri::command]
pub fn capture_rect_to_file(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    window_id: u32,
    role: String,
    path: String,
) -> Result<(), String> {
    let mut command = Command::new("screencapture");
    // 파일 저장: -c 없이 경로만 지정
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
    Ok(())
}
```

변경 없음: `capture_rect` (클립보드 복사) — 그대로 유지

#### polling.rs — handle_click_capture 수정

```rust
fn handle_click_capture(handle: &AppHandle, state: &tauri::State<AppState>) {
    state.is_snip_active.store(false, Ordering::Relaxed);

    if let Some(win) = handle.get_webview_window(WINDOW_LABEL_MAIN) {
        let _ = win.hide();
    }

    thread::sleep(Duration::from_millis(WINDOW_HIDE_DELAY_MS));

    let rect_to_capture = state.current_info.lock()
        .map(|lock| lock.clone())
        .ok()
        .flatten();

    // 변경: capture_rect 직접 호출 → 이벤트 emit으로 교체
    if let Some(info) = rect_to_capture {
        let _ = handle.emit("capture-click", info);
    }
}
```

#### lib.rs — 커맨드 및 트레이 메뉴 수정

```rust
// invoke_handler에 capture_rect_to_file 추가
.invoke_handler(tauri::generate_handler![
    capture::capture_rect,
    capture::capture_rect_to_file,  // 추가
    hide_window
])

// 트레이 메뉴에 "Settings" 항목 추가
let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
let menu = Menu::with_items(app, &[&snip_i, &settings_i, &quit_i])?;

// on_menu_event에 settings 처리 추가
"settings" => {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL_MAIN) {
        let _ = win.show();
        let _ = win.set_focus();
    }
    let _ = app.emit("show-settings", ());
},
```

#### constants.rs — 이벤트 상수 추가

```rust
pub const EVENT_CAPTURE_CLICK: &str = "capture-click";
pub const EVENT_SHOW_SETTINGS: &str = "show-settings";
```

### 5-2. Frontend (App.tsx)

#### 상태 구조

```typescript
// 기존
const [highlight, setHighlight] = useState<UIElementInfo | null>(null);

// 추가
const [showSettings, setShowSettings] = useState(false);
const [copyToClipboard, setCopyToClipboard] = useState(
  () => localStorage.getItem("xray-copy-to-clipboard") !== "false"
);
```

#### 이벤트 리스너 (capture-click)

```typescript
useEffect(() => {
  const unlistenPromise = listen<UIElementInfo>("capture-click", async (event) => {
    const info = event.payload;

    // macOS 저장 다이얼로그 (NSSavePanel)
    const path = await save({
      defaultPath: `capture-${Date.now()}.png`,
      filters: [{ name: "PNG Image", extensions: ["png"] }],
    });

    if (!path) return; // 취소

    await invoke("capture_rect_to_file", {
      x: info.globalX,
      y: info.globalY,
      width: info.width,
      height: info.height,
      windowId: info.windowId,
      role: info.role,
      path,
    });

    if (copyToClipboard) {
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
}, [copyToClipboard]);
```

#### 이벤트 리스너 (show-settings)

```typescript
useEffect(() => {
  const unlistenPromise = listen("show-settings", () => {
    setShowSettings((prev) => !prev);
  });
  return () => { unlistenPromise.then((u) => u()); };
}, []);
```

#### 설정 패널 UI

```
┌─────────────────────────────────┐
│  xray Settings              [×] │
├─────────────────────────────────┤
│  Capture                        │
│  ☑  Copy to clipboard           │
│     좌클릭 시 클립보드에도 저장  │
└─────────────────────────────────┘
```

위치: 화면 우하단 고정 (position: fixed, bottom: 20px, right: 20px)
크기: 260px × auto
스타일: 기존 Info HUD와 동일한 다크 테마 (`#cc0000` 포인트 컬러)

```typescript
{showSettings && (
  <div style={settingsPanelStyle}>
    <div style={settingsHeaderStyle}>
      <span>xray Settings</span>
      <button onClick={() => setShowSettings(false)}>×</button>
    </div>
    <div style={settingsBodyStyle}>
      <label style={checkboxLabelStyle}>
        <input
          type="checkbox"
          checked={copyToClipboard}
          onChange={(e) => {
            setCopyToClipboard(e.target.checked);
            localStorage.setItem("xray-copy-to-clipboard", String(e.target.checked));
          }}
        />
        Copy to clipboard
      </label>
    </div>
  </div>
)}
```

#### 포인터 이벤트 주의사항

현재 App의 최상위 div는 클릭이 하위 앱으로 투과되도록 설정된다.
설정 패널에는 `pointerEvents: "auto"`를 명시적으로 설정해야 클릭이 동작한다.

### 5-3. tauri.conf.json — 권한 설정

`tauri-plugin-dialog`는 기본 사용 시 별도 권한 선언이 필요하다.

```json
{
  "app": {
    "security": {
      "capabilities": [
        {
          "identifier": "default",
          "description": "default",
          "windows": ["main"],
          "permissions": [
            "core:default",
            "dialog:allow-save"
          ]
        }
      ]
    }
  }
}
```

> 현재 `"csp": null` 설정은 capabilities 방식으로 교체해야 한다.

## 6. 변경 파일 목록 (최종 확정)

| 파일 | 변경 내용 | 변경량 |
|------|-----------|--------|
| `src-tauri/src/capture.rs` | `capture_rect_to_file` 함수 추가 | +25줄 |
| `src-tauri/src/polling.rs` | `handle_click_capture`: emit 방식으로 변경 | ~5줄 수정 |
| `src-tauri/src/lib.rs` | 커맨드 등록, Settings 트레이 메뉴 | ~10줄 |
| `src-tauri/src/constants.rs` | 이벤트 상수 2개 추가 | +2줄 |
| `src-tauri/tauri.conf.json` | dialog 권한, capabilities 구조로 변경 | ~10줄 |
| `src-tauri/Cargo.toml` | ❌ 변경 없음 (dialog 이미 존재) | 0줄 |
| `src/App.tsx` | 인터페이스 확장, 이벤트 리스너 2개, 설정 패널 UI | +80줄 |
| `package.json` | `@tauri-apps/plugin-dialog` 추가 | +1줄 |

## 7. 구현 순서

```
1. constants.rs      — 이벤트 상수 추가 (의존성 없음)
2. capture.rs        — capture_rect_to_file 추가 (독립)
3. polling.rs        — handle_click_capture emit으로 변경
4. lib.rs            — 커맨드 등록, Settings 트레이
5. tauri.conf.json   — dialog 권한 설정
6. package.json      — @tauri-apps/plugin-dialog 추가
7. App.tsx           — 인터페이스, 이벤트 리스너, 설정 패널
8. npm install       — 의존성 설치
9. cargo build       — 빌드 확인
```

## 8. 검증 기준 (Success Criteria)

| ![[Pasted image 20260219132952.png]]![[Pasted image 20260219132953.png]]# | 검증 항목                     | 방법                      |
| ------------------------------------------------------------------------- | ------------------------- | ----------------------- |
| 1                                                                         | 좌클릭 시 macOS 저장 다이얼로그 열림   | 직접 테스트                  |
| 2                                                                         | 지정 경로에 PNG 파일 저장          | `ls` 확인                 |
| 3                                                                         | 트레이 "Settings" → 설정 패널 열림 | 직접 테스트                  |
| 4                                                                         | 체크 ON: 파일 + 클립보드 저장       | 붙여넣기 확인                 |
| 5                                                                         | 체크 OFF: 파일만 저장 (클립보드 없음)  | 붙여넣기 실패 확인              |
| 6                                                                         | 설정값 재시작 후 유지              | 앱 재시작 후 확인              |
| 7                                                                         | 다이얼로그 취소 → 아무것도 안 함       | 직접 테스트                  |
| 8                                                                         | 기존 클립보드 복사 로직 영향 없음       | `capture_rect` 단독 호출 확인 |
