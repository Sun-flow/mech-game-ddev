use macroquad::prelude::*;

/// Collected per-frame mouse state, constructed once in the main loop.
#[allow(dead_code)]
pub struct MouseState {
    /// Mouse position in screen coordinates.
    pub screen_mouse: Vec2,
    /// Mouse position in world coordinates (camera-transformed).
    pub world_mouse: Vec2,
    /// Mouse button pressed this frame (single-fire).
    pub left_click: bool,
    pub right_click: bool,
    pub middle_click: bool,
    /// Mouse button held down (continuous).
    pub left_down: bool,
    pub middle_down: bool,
    /// Mouse wheel vertical delta.
    pub scroll: f32,
}
