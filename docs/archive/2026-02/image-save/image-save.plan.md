# Plan: image-save

## 1. Overview

**Feature Name**: image-save
**Repository**: https://github.com/jsseoi/xray (fork of wlswo/xray)
**Branch**: feature/image-save (worktree: /Users/jsseoi/workspace/xray-image-save)
**Date**: 2026-02-19
**Phase**: Plan (v2 - 요구사항 변경 반영)

## 2. Background & Motivation

현재 xray는 UI 요소 캡처 시 **클립보드에만** 복사(`screencapture -c`)한다.
파일로 저장하는 기능이 전혀 없어, 캡처 결과를 파일로 보관할 수 없다.

### 현재 구현 상태

| 항목 | 현황 |
|------|------|
| 클립보드 복사 (좌클릭) | ✅ 구현됨 (`capture.rs` → `screencapture -c`) |
| 파일로 저장 | ❌ 미구현 |
| 설정 패널 | ❌ 미구현 |

## 3. Goals (v2 확정)

1. **좌클릭 시** macOS 네이티브 저장 다이얼로그(NSSavePanel)로 파일 저장
2. **설정 패널** 추가: 체크박스 "좌클릭 시 클립보드에 저장" (기본값: 체크됨)
   - 체크됨: 좌클릭 → 클립보드에도 함께 저장
   - 체크 해제: 좌클릭 → 파일 저장만 (클립보드 저장 안 함)

### 제거된 항목 (v1 → v2)
- ~~다양한 포맷 지원 (PNG/JPEG/TIFF/PDF/BMP)~~ → 제거 (macOS 기본 저장 UI에 위임)
- ~~우클릭 메뉴 방식~~ → 제거

## 4. Scope

### In Scope
- 좌클릭 시 macOS 네이티브 저장 다이얼로그 호출 (Tauri dialog plugin)
- 설정 패널 UI (체크박스 1개)
- 설정 영속화 (localStorage 또는 Tauri store)
- 기존 `capture_rect` 활용 + 새 `capture_rect_to_file` 커맨드 추가

### Out of Scope
- 포맷 선택 UI
- 우클릭 메뉴
- Windows/Linux 지원

## 5. User Flow

```
[Cmd+Shift+X] 단축키
→ 오버레이 활성화 (십자 커서)
→ 요소 위 호버 → 빨간 박스 표시
→ 좌클릭
  ├─ macOS 저장 다이얼로그 열림 (파일명/위치 선택)
  │   └─ 저장 확인
  │       ├─ 설정 "클립보드에 저장" ON → 파일 저장 + 클립보드 복사
  │       └─ 설정 "클립보드에 저장" OFF → 파일 저장만
  └─ 저장 취소 → 오버레이 유지 (재시도 가능)

[Cmd+,] 또는 트레이 메뉴 "Settings"
→ 설정 패널 열림
  └─ [☑] 좌클릭 시 클립보드에 저장 (기본: 체크)
```

## 6. Files to Modify

| 파일 | 변경 유형 | 설명 |
|------|-----------|------|
| `src-tauri/src/capture.rs` | 수정 | `capture_rect_to_file(path)` 커맨드 추가 |
| `src-tauri/src/lib.rs` | 수정 | dialog 플러그인 등록, 새 커맨드 등록 |
| `src-tauri/Cargo.toml` | 수정 | `tauri-plugin-dialog` 의존성 추가 |
| `src-tauri/tauri.conf.json` | 수정 | dialog 플러그인 권한 설정 |
| `src/App.tsx` | 수정 | 클릭 핸들러 + 설정 패널 UI |
| `package.json` | 수정 | `@tauri-apps/plugin-dialog` 추가 |

## 7. Technical Details

### Backend: capture_rect_to_file
```rust
#[tauri::command]
pub fn capture_rect_to_file(
    x: f64, y: f64, width: f64, height: f64,
    window_id: u32, role: String,
    path: String,  // 저장 경로 (다이얼로그에서 받아옴)
) -> Result<(), String> {
    let mut command = Command::new("screencapture");
    // 포맷은 path 확장자에서 자동 결정 (macOS 저장 UI가 처리)
    if role.contains("Window") && window_id > 0 {
        command.arg("-l").arg(window_id.to_string());
    } else {
        command.arg("-R").arg(format!("{},{},{},{}", x, y, width, height));
    }
    command.arg(&path);
    // ...
}
```

### Frontend: 설정 상태 관리
```typescript
// 설정: localStorage에 영속화
const [copyToClipboard, setCopyToClipboard] = useState(
  () => localStorage.getItem("copyToClipboard") !== "false"
);

// 클릭 핸들러
async function handleCapture(element: UIElementInfo) {
  const path = await save({ filters: [{ name: "Image", extensions: ["png"] }] });
  if (!path) return; // 취소

  await invoke("capture_rect_to_file", { ...element, path });
  if (copyToClipboard) {
    await invoke("capture_rect", { ...element }); // 클립보드 복사
  }
}
```

## 8. Success Criteria

- [ ] 좌클릭 시 macOS 저장 다이얼로그 열림
- [ ] 지정된 경로에 파일 저장 성공
- [ ] 설정 패널에 체크박스 표시 (기본값: 체크됨)
- [ ] 체크됨: 파일 저장 + 클립보드 복사 동시 동작
- [ ] 체크 해제: 파일 저장만 동작
- [ ] 설정값이 앱 재시작 후에도 유지됨
- [ ] 저장 다이얼로그 취소 시 오버레이 유지

## 9. Risks

| 위험 | 완화 방안 |
|------|-----------|
| 저장 다이얼로그가 오버레이 위로 안 뜰 수 있음 | Tauri window focus/z-order 처리 |
| `screencapture` 실행 타이밍 (다이얼로그 닫히기 전) | path 받은 후 invoke 순서 보장 |
