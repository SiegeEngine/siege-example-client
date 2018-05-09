
mod decorate;

use ui::Rect;

#[derive(Debug, Clone)]
pub struct UiWindow {
    pub rect: Rect,
    pub color: [f32; 4],
    pub child_alpha: f32,
}

impl UiWindow {
    pub fn new(rect: Rect, color: [f32; 4], child_alpha: f32)
               -> UiWindow
    {
        UiWindow {
            rect: rect,
            color: color,
            child_alpha: child_alpha,
        }
    }

    pub fn get_color(&self) -> [f32; 4] {
        self.color
    }
}
