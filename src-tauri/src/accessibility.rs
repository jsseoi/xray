use crate::constants::{ax_attributes, ax_roles, ACCESSIBILITY_RECURSION_LIMIT};
use accessibility_sys::{
    kAXErrorSuccess, AXUIElementCopyAttributeValue, AXUIElementCopyElementAtPosition,
    AXUIElementCreateSystemWide, AXUIElementRef, AXValueGetValue, AXValueRef,
};
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_graphics::geometry::{CGPoint, CGSize};
use std::ffi::c_void;
use std::ptr;

/// Represents the geometry and metadata of a UI element found via accessibility APIs.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
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
    /// A private macOS API to get the window ID from an AXElement.
    fn _AXUIElementGetWindow(element: AXUIElementRef, id: *mut u32) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(theArray: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(theArray: *const c_void, idx: isize) -> *const c_void;
}

/// Checks if the left mouse button is currently pressed.
///
/// Uses `CGEventSourceButtonState` to query the HID system state.
pub fn is_mouse_left_down() -> bool {
    unsafe {
        // kCGEventSourceStateHIDSystemState = 1, kCGMouseButtonLeft = 0
        CGEventSourceButtonState(1, 0)
    }
}

/// Finds the UI element at the current mouse cursor position.
///
/// This function performs the following steps:
/// 1. Gets the current mouse location.
/// 2. Queries the system-wide accessibility object for the element at that location.
/// 3. Drills down into the element hierarchy to find the most specific leaf node.
/// 4. Extracts position, size, and role information.
pub fn get_element_at_mouse() -> Option<UIElementInfo> {
    unsafe {
        let source = core_graphics::event_source::CGEventSource::new(
            core_graphics::event_source::CGEventSourceStateID::HIDSystemState,
        )
        .ok()?;
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

        // Deep Drill Down: Search as deep as possible to find leaf nodes like <img>.
        for _ in 0..ACCESSIBILITY_RECURSION_LIMIT {
            if let Some(child) = drill_down(element_ref, mouse_loc.x, mouse_loc.y) {
                // Release the parent and move to the child.
                core_foundation::base::CFRelease(element_ref as *const c_void);
                element_ref = child;
            } else {
                break; // No more children found, stop recursion.
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
                role,
            })
        } else {
            None
        }
    }
}

/// Drills down into a container element to find a more specific child under the mouse coordinates.
unsafe fn drill_down(element: AXUIElementRef, mx: f64, my: f64) -> Option<AXUIElementRef> {
    let attr_name = CFString::new(ax_attributes::CHILDREN);
    let mut value_ref: *const c_void = ptr::null();

    let result =
        AXUIElementCopyAttributeValue(element, attr_name.as_concrete_TypeRef(), &mut value_ref);

    if result == kAXErrorSuccess && !value_ref.is_null() {
        let count = CFArrayGetCount(value_ref);

        let mut best_child: Option<AXUIElementRef> = None;
        let mut min_area = f64::MAX;

        // Iterate through all children to find the best fit.
        for i in 0..count {
            let child_ptr = CFArrayGetValueAtIndex(value_ref, i) as AXUIElementRef;

            if let (Some((cx, cy)), Some((cw, ch))) = (get_position(child_ptr), get_size(child_ptr))
            {
                // Hit Test: Check if mouse is within bounds
                if mx >= cx && mx < cx + cw && my >= cy && my < cy + ch {
                    let area = cw * ch;
                    let mut should_update = false;

                    // Strategy: Prefer smaller areas (more specific elements)
                    if area < min_area {
                        should_update = true;
                    }
                    // Tie-breaking: If areas are similar, use role priority
                    else if (area - min_area).abs() < f64::EPSILON {
                        should_update = should_update_based_on_role(best_child, child_ptr);
                    }

                    if should_update {
                        min_area = area;
                        best_child = Some(child_ptr);
                    }
                }
            }
        }

        // Retain the best child before releasing the array
        if let Some(child) = best_child {
            core_foundation::base::CFRetain(child as *const c_void);
            core_foundation::base::CFRelease(value_ref);
            return Some(child);
        }

        core_foundation::base::CFRelease(value_ref);
    }
    None
}

/// Helper function to decide if we should switch to the new child based on AXRole priority.
unsafe fn should_update_based_on_role(
    current_best: Option<AXUIElementRef>,
    new_candidate: AXUIElementRef,
) -> bool {
    let Some(best) = current_best else {
        return true;
    };

    let best_role = get_role(best).unwrap_or_default();
    let new_role = get_role(new_candidate).unwrap_or_default();

    // Tier 1: Visual final elements (Images, Checkboxes)
    let is_tier_1 = |r: &str| {
        matches!(
            r,
            ax_roles::IMAGE | ax_roles::CHECKBOX | ax_roles::RADIO_BUTTON
        )
    };

    // Tier 2: Text or Interactive elements (Text, Buttons)
    let is_tier_2 = |r: &str| {
        matches!(
            r,
            ax_roles::STATIC_TEXT | ax_roles::HEADING | ax_roles::BUTTON
        )
    };

    // Tier 3: Containers (Groups, Areas)
    let is_tier_3 = |r: &str| {
        matches!(
            r,
            ax_roles::LINK | ax_roles::GROUP | ax_roles::WEB_AREA | ax_roles::SCROLL_AREA
        )
    };

    if is_tier_1(&new_role) && !is_tier_1(&best_role) {
        return true;
    }
    if is_tier_2(&new_role) && is_tier_3(&best_role) {
        return true;
    }
    if is_tier_1(&new_role) && is_tier_1(&best_role) {
        return true;
    }

    false
}

unsafe fn get_role(element: AXUIElementRef) -> Option<String> {
    get_string_attribute(element, ax_attributes::ROLE)
}

unsafe fn get_position(element: AXUIElementRef) -> Option<(f64, f64)> {
    let attr_name = CFString::new(ax_attributes::POSITION);
    let mut value_ref: *const c_void = ptr::null();

    let result =
        AXUIElementCopyAttributeValue(element, attr_name.as_concrete_TypeRef(), &mut value_ref);

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

unsafe fn get_size(element: AXUIElementRef) -> Option<(f64, f64)> {
    let attr_name = CFString::new(ax_attributes::SIZE);
    let mut value_ref: *const c_void = ptr::null();

    let result =
        AXUIElementCopyAttributeValue(element, attr_name.as_concrete_TypeRef(), &mut value_ref);

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

/// Helper to get a string attribute from an AX element.
unsafe fn get_string_attribute(element: AXUIElementRef, attribute: &str) -> Option<String> {
    let attr_name = CFString::new(attribute);
    let mut value_ref: *const c_void = ptr::null();

    let result =
        AXUIElementCopyAttributeValue(element, attr_name.as_concrete_TypeRef(), &mut value_ref);

    if result == kAXErrorSuccess && !value_ref.is_null() {
        let cf_str = value_ref as core_foundation::string::CFStringRef;
        let ret_str = CFString::wrap_under_get_rule(cf_str).to_string();
        core_foundation::base::CFRelease(value_ref);
        return Some(ret_str);
    }
    None
}
