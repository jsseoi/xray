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
    /// A private macOS API to get the window ID from an AXElement.
    fn _AXUIElementGetWindow(element: AXUIElementRef, id: *mut u32) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(theArray: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(theArray: *const c_void, idx: isize) -> *const c_void;
}

/// Checks if the left mouse button is pressed.
pub fn is_mouse_left_down() -> bool {
    unsafe {
        // kCGEventSourceStateHIDSystemState = 1, kCGMouseButtonLeft = 0
        CGEventSourceButtonState(1, 0)
    }
}

/// Finds the UI element at the mouse cursor's position.
pub fn get_element_at_mouse() -> Option<UIElementInfo> {
    unsafe {
        // Get the current mouse coordinates.
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

        // Deep Drill Down
        // To find leaf nodes like <img>, we search as deep as possible (up to 50 levels).
        for _ in 0..50 { 
            if let Some(child) = drill_down(element_ref, mouse_loc.x, mouse_loc.y) {
                // Release the parent and move to the child.
                core_foundation::base::CFRelease(element_ref as *const c_void);
                element_ref = child;
            } else {
                break; // If there are no more children, exit.
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

/// Drills down into a container element to find a more specific child under the mouse.
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

        // Iterate through all children to find the best fit.
        for i in 0..count {
             let child_ptr = CFArrayGetValueAtIndex(value_ref, i) as AXUIElementRef;
             
             // Check the child element's position and size.
             if let (Some((cx, cy)), Some((cw, ch))) = (get_position(child_ptr), get_size(child_ptr)) {
                 // Check if the mouse is inside the child element (Hit Test).
                 if mx >= cx && mx < cx + cw && my >= cy && my < cy + ch {
                     let area = cw * ch;
                     
                     let mut update = false;

                     // [Condition 1] If the area is smaller, it's always considered a more specific child element, so replace.
                     if area < min_area {
                         update = true;
                     } 
                     // [Condition 2] If areas are equal (overlapping) or very similar -> Decide based on Role priority.
                     else if area == min_area {
                         if let Some(best) = best_child {
                             let best_role = get_role(best).unwrap_or_default();
                             let curr_role = get_role(child_ptr).unwrap_or_default();
                             
                             // Tier 1: Visual final elements like images, checkboxes, etc.
                             let is_tier_1 = |r: &str| r == "AXImage" || r == "AXCheckBox" || r == "AXRadioButton";
                             
                             // Tier 2: Text or buttons.
                             let is_tier_2 = |r: &str| r == "AXStaticText" || r == "AXHeading" || r == "AXButton";
                             
                             // Tier 3: Containers (links, groups, etc.).
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
        
        // Retain the final selected child and release the array.
        if let Some(child) = best_child {
            core_foundation::base::CFRetain(child as *const c_void);
            core_foundation::base::CFRelease(value_ref);
            return Some(child);
        }
        
        core_foundation::base::CFRelease(value_ref);
    }
    None
}

/// Gets the role of the UI element.
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

/// Gets the global position of the UI element.
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

/// Gets the size (width, height) of the UI element.
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
