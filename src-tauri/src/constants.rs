/// The label/ID of the main overlay window.
pub const WINDOW_LABEL_MAIN: &str = "main";

/// The event name emitted to the frontend when a UI element is hovered.
pub const EVENT_ELEMENT_HOVER: &str = "element-hover";

/// The polling interval in milliseconds (approx. 60 FPS).
pub const POLLING_INTERVAL_MS: u64 = 16;

/// The delay in milliseconds to wait for the window to hide before capturing the screen.
pub const WINDOW_HIDE_DELAY_MS: u64 = 150;

/// The maximum depth to drill down into accessibility elements.
pub const ACCESSIBILITY_RECURSION_LIMIT: i32 = 50;

/// Accessibility attribute names.
pub mod ax_attributes {
    pub const CHILDREN: &str = "AXChildren";
    pub const ROLE: &str = "AXRole";
    pub const POSITION: &str = "AXPosition";
    pub const SIZE: &str = "AXSize";
}

/// Accessibility roles.
pub mod ax_roles {
    pub const IMAGE: &str = "AXImage";
    pub const CHECKBOX: &str = "AXCheckBox";
    pub const RADIO_BUTTON: &str = "AXRadioButton";
    pub const STATIC_TEXT: &str = "AXStaticText";
    pub const HEADING: &str = "AXHeading";
    pub const BUTTON: &str = "AXButton";
    pub const LINK: &str = "AXLink";
    pub const GROUP: &str = "AXGroup";
    pub const WEB_AREA: &str = "AXWebArea";
    pub const SCROLL_AREA: &str = "AXScrollArea";
}
