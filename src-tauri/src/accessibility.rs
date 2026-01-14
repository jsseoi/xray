use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_graphics::geometry::{CGPoint, CGSize};
use accessibility_sys::{
    AXUIElementCreateSystemWide, AXUIElementCopyElementAtPosition,
    AXUIElementCopyAttributeValue, AXValueGetValue,
    kAXErrorSuccess, AXUIElementRef, AXValueRef,
};
use std::ffi::c_void;
use std::ptr;

#[derive(Clone, Debug, serde::Serialize)]
pub struct UIElementInfo {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub global_x: f64,
    pub global_y: f64,
    pub window_id: u32,
    pub role: String,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceButtonState(stateID: u32, button: u32) -> bool;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    /// 요소(AXElement)로부터 윈도우 ID를 가져오는 macOS 비공개 API
    fn _AXUIElementGetWindow(element: AXUIElementRef, id: *mut u32) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(theArray: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(theArray: *const c_void, idx: isize) -> *const c_void;
}

/// 마우스 왼쪽 버튼이 눌려 있는지 확인합니다.
pub fn is_mouse_left_down() -> bool {
    unsafe {
        // kCGEventSourceStateHIDSystemState = 1, kCGMouseButtonLeft = 0
        CGEventSourceButtonState(1, 0)
    }
}

/// 마우스 커서 위치에 있는 UI 요소를 탐색합니다.
pub fn get_element_at_mouse() -> Option<UIElementInfo> {
    unsafe {
        // 현재 마우스 좌표 가져오기
        let source = core_graphics::event_source::CGEventSource::new(core_graphics::event_source::CGEventSourceStateID::HIDSystemState).ok()?;
        let event = core_graphics::event::CGEvent::new(source).ok()?;
        let mouse_loc = event.location();

        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }

        let mut element_ref: AXUIElementRef = ptr::null_mut();
        let result = AXUIElementCopyElementAtPosition(
            system_wide,
            mouse_loc.x as f32,
            mouse_loc.y as f32,
            &mut element_ref,
        );

        if result != kAXErrorSuccess || element_ref.is_null() {
            if !element_ref.is_null() {
                core_foundation::base::CFRelease(element_ref as *const c_void);
            }
            return None;
        }

        // 심층 탐색 (Deep Drill Down)
        // <img>와 같은 리프 노드를 찾기 위해 가능한 깊이(최대 50단계) 탐색합니다.
        for _ in 0..50 { 
            if let Some(child) = drill_down(element_ref, mouse_loc.x, mouse_loc.y) {
                // 부모를 해제하고 자식으로 이동
                core_foundation::base::CFRelease(element_ref as *const c_void);
                element_ref = child;
            } else {
                break; // 더 이상 하위 요소가 없으면 종료
            }
        }

        let pos = get_position(element_ref);
        let size = get_size(element_ref);
        let role = get_role(element_ref).unwrap_or_else(|| "Unknown".to_string());
        
        let mut window_id: u32 = 0;
        let _ = _AXUIElementGetWindow(element_ref, &mut window_id);

        core_foundation::base::CFRelease(element_ref as *const c_void);

        if let (Some((x, y)), Some((w, h))) = (pos, size) {
             Some(UIElementInfo { 
                 x, 
                 y, 
                 width: w, 
                 height: h,
                 global_x: x,
                 global_y: y,
                 window_id,
                 role
             })
        } else {
            None
        }
    }
}

/// 컨테이너 요소 내부를 탐색하여 마우스 아래에 있는 더 구체적인 자식을 찾습니다.
unsafe fn drill_down(element: AXUIElementRef, mx: f64, my: f64) -> Option<AXUIElementRef> {
    let attr_name = CFString::new("AXChildren");
    let mut value_ref: *const c_void = ptr::null();
    
    let result = AXUIElementCopyAttributeValue(
        element,
        attr_name.as_concrete_TypeRef(),
        &mut value_ref,
    );

    if result == kAXErrorSuccess && !value_ref.is_null() {
        let count = CFArrayGetCount(value_ref);
        
        let mut best_child: Option<AXUIElementRef> = None;
        let mut min_area = f64::MAX;

        // 모든 자식 요소를 순회하여 가장 적합한 요소를 찾습니다.
        for i in 0..count {
             let child_ptr = CFArrayGetValueAtIndex(value_ref, i) as AXUIElementRef;
             
             // 자식 요소의 위치와 크기 확인
             if let (Some((cx, cy)), Some((cw, ch))) = (get_position(child_ptr), get_size(child_ptr)) {
                 // 마우스가 해당 자식 요소 안에 있는지 확인 (Hit Test)
                 if mx >= cx && mx < cx + cw && my >= cy && my < cy + ch {
                     let area = cw * ch;
                     
                     let mut update = false;

                     // [조건 1] 면적이 더 작으면 무조건 더 구체적인 자식 요소로 판단하여 교체
                     if area < min_area {
                         update = true;
                     } 
                     // [조건 2] 면적이 같거나(겹침) 매우 유사할 경우 -> Role 우선순위로 결정
                     else if area == min_area {
                         if let Some(best) = best_child {
                             let best_role = get_role(best).unwrap_or_default();
                             let curr_role = get_role(child_ptr).unwrap_or_default();
                             
                             // 1순위: 이미지, 체크박스 등 시각적 최종 요소
                             let is_tier_1 = |r: &str| r == "AXImage" || r == "AXCheckBox" || r == "AXRadioButton";
                             
                             // 2순위: 텍스트나 버튼
                             let is_tier_2 = |r: &str| r == "AXStaticText" || r == "AXHeading" || r == "AXButton";
                             
                             // 3순위: 컨테이너 (링크, 그룹 등)
                             let is_tier_3 = |r: &str| r == "AXLink" || r == "AXGroup" || r == "AXWebArea" || r == "AXScrollArea";

                             if is_tier_1(&curr_role) && !is_tier_1(&best_role) {
                                 update = true; 
                             } else if is_tier_2(&curr_role) && is_tier_3(&best_role) {
                                 update = true; 
                             } else if is_tier_1(&curr_role) && is_tier_1(&best_role) {
                                 update = true; 
                             }
                         } else {
                             update = true;
                         }
                     }

                     if update {
                         min_area = area;
                         best_child = Some(child_ptr);
                     }
                 }
             }
        }
        
        // 최종 선택된 자식을 Retain 하고, 배열은 Release
        if let Some(child) = best_child {
            core_foundation::base::CFRetain(child as *const c_void);
            core_foundation::base::CFRelease(value_ref);
            return Some(child);
        }
        
        core_foundation::base::CFRelease(value_ref);
    }
    None
}

/// UI 요소의 역할(Role)을 가져옵니다.
unsafe fn get_role(element: AXUIElementRef) -> Option<String> {
    let attr_name = CFString::new("AXRole");
    let mut value_ref: *const c_void = ptr::null();
    
    let result = AXUIElementCopyAttributeValue(
        element,
        attr_name.as_concrete_TypeRef(),
        &mut value_ref,
    );

    if result == kAXErrorSuccess && !value_ref.is_null() {
         let cf_str = value_ref as core_foundation::string::CFStringRef;
         let role_str = CFString::wrap_under_get_rule(cf_str).to_string();
         core_foundation::base::CFRelease(value_ref);
         return Some(role_str);
    }
    None
}

/// UI 요소의 전역 위치를 가져옵니다.
unsafe fn get_position(element: AXUIElementRef) -> Option<(f64, f64)> {
    let attr_name = CFString::new("AXPosition");
    let mut value_ref: *const c_void = ptr::null();
    
    let result = AXUIElementCopyAttributeValue(
        element,
        attr_name.as_concrete_TypeRef(),
        &mut value_ref, 
    );

    if result == kAXErrorSuccess && !value_ref.is_null() {
         let val = value_ref as AXValueRef;
         let mut point = CGPoint::default();
         // kAXValueCGPointType = 1
         let success = AXValueGetValue(val, 1, &mut point as *mut _ as *mut c_void);
         core_foundation::base::CFRelease(value_ref);
         if success {
             return Some((point.x, point.y));
         }
    }
    None
}

/// UI 요소의 크기(너비, 높이)를 가져옵니다.
unsafe fn get_size(element: AXUIElementRef) -> Option<(f64, f64)> {
    let attr_name = CFString::new("AXSize");
    let mut value_ref: *const c_void = ptr::null();
    
    let result = AXUIElementCopyAttributeValue(
        element,
        attr_name.as_concrete_TypeRef(),
        &mut value_ref,
    );

    if result == kAXErrorSuccess && !value_ref.is_null() {
         let val = value_ref as AXValueRef;
         let mut size = CGSize::default();
         // kAXValueCGSizeType = 2
         let success = AXValueGetValue(val, 2, &mut size as *mut _ as *mut c_void);
         core_foundation::base::CFRelease(value_ref);
         if success {
             return Some((size.width, size.height));
         }
    }
    None
}
