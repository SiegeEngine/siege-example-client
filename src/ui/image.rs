
use ui::geom::{Rect, AbsRect};

#[derive(Debug, Clone)]
pub struct UiImage {
    // FIXME: don't have a COPY of the entire widget here, use some referencing system.
    pub widget: AbsRect,

    /// Where on the screen should the widget be pinned (mapped and streched to fit)
    pub widget_pin_rect: Rect,

    /// Where on the screen should we draw (entire space, may be more or less than
    /// the mapped rect, widget will 'repeat')
    pub screen_draw_rect: Rect,

}

pub const WINDOW_TL_CORNER: AbsRect =
    AbsRect { x: 0.0, y: 0.0, width: 12.0, height: 12.0 };
pub const WINDOW_TR_CORNER: AbsRect =
    AbsRect { x: 24.0, y: 0.0, width: 12.0, height: 12.0 };
pub const WINDOW_BL_CORNER: AbsRect =
    AbsRect { x: 0.0, y: 36.0, width: 12.0, height: 12.0 };
pub const WINDOW_BR_CORNER: AbsRect =
    AbsRect { x: 24.0, y: 36.0, width: 12.0, height: 12.0 };
pub const WINDOW_TOP: AbsRect =
    AbsRect { x: 12.0, y: 0.0, width: 12.0, height: 12.0 };
pub const WINDOW_BOTTOM: AbsRect =
    AbsRect { x: 12.0, y: 36.0, width: 12.0, height: 12.0 };
pub const WINDOW_LEFT: AbsRect =
    AbsRect { x: 0.0, y: 12.0, width: 12.0, height: 12.0 };
pub const WINDOW_RIGHT: AbsRect =
    AbsRect { x: 24.0, y: 12.0, width: 12.0, height: 12.0 };
pub const WINDOW_PANE: AbsRect =
    AbsRect { x: 12.0, y: 12.0, width: 12.0, height: 12.0 };
#[allow(dead_code)]
pub const WINDOW_L_RULE: AbsRect =
    AbsRect { x: 0.0, y: 24.0, width: 12.0, height: 12.0 };
#[allow(dead_code)]
pub const WINDOW_R_RULE: AbsRect =
    AbsRect { x: 24.0, y: 24.0, width: 12.0, height: 12.0 };
#[allow(dead_code)]
pub const WINDOW_RULE: AbsRect =
    AbsRect { x: 12.0, y: 24.0, width: 12.0, height: 12.0 };
