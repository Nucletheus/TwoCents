//! Design tokens — the language layer.
//!
//! One source of truth for spacing, type, geometry, and the Notion-vertical
//! component primitives. Themes only contribute colors; everything structural
//! lives here. Constants are `pub` so other modules can import them, but
//! nothing in this file ever changes at runtime.
use eframe::egui::CornerRadius;

// Design-system values. Changing any of them is a one-line edit that
// propagates to every component builder that uses it.

// ---- Spacing (4-base scale, in points) ----
pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_5: f32 = 24.0;
pub const SPACE_6: f32 = 40.0;
pub const SPACE_7: f32 = 64.0;

// ---- Geometry ----
pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;
pub const BORDER_W: f32 = 1.0;
pub const BORDER_W_STRONG: f32 = 2.0;
pub const FOCUS_RING_W: f32 = 2.0;

// ---- Grid (preserved from the existing app, not Notion-derived) ----
pub const GRID_HEADER_HEIGHT: f32 = 28.0;
pub const GRID_ROW_HEIGHT: f32 = 24.0;
pub const GRID_SELECTION_STATUS_HEIGHT: f32 = 18.0;

// ---- Component primitives ----
//
// These are the only places that hardcode chrome. Anything that wants a card,
// button, modal, or input should call these, not build its own Frame.

pub fn radius_sm() -> CornerRadius {
    CornerRadius::same(RADIUS_SM as u8)
}
pub fn radius_md() -> CornerRadius {
    CornerRadius::same(RADIUS_MD as u8)
}
pub fn radius_lg() -> CornerRadius {
    CornerRadius::same(RADIUS_LG as u8)
}
