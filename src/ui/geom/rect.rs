
use super::{AbsRect, Coord, Dim};

// Specify horizontal elements of a viewport in one of three ways
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RectX {
    LeftRight(Coord, Coord),
    LeftWidth(Coord, Dim),
    RightWidth(Coord, Dim),
}


// Specify vertical elements of a viewport in one of three ways
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RectY {
    TopBottom(Coord, Coord),
    TopHeight(Coord, Dim),
    BottomHeight(Coord, Dim),
}

// A viewport in coordinates relative to some parent
#[derive(Debug, Clone)]
pub struct Rect {
    pub rectx: RectX,
    pub recty: RectY,
}

impl Rect {
    pub fn new(rectx: RectX, recty: RectY) -> Rect {
        Rect {
            rectx: rectx,
            recty: recty
        }
    }

    pub fn absolute(&self, abs_parent: &AbsRect) -> AbsRect {
        let mut absrect = AbsRect {
            x: match self.rectx {
                RectX::LeftRight(ref left, _) => left.absolute_xpoint(abs_parent),
                RectX::LeftWidth(ref left, _) => left.absolute_xpoint(abs_parent),
                RectX::RightWidth(ref right, ref width) =>
                    right.absolute_xpoint(abs_parent) - width.absolute_xdim(abs_parent),
            },
            y: match self.recty {
                RectY::TopBottom(ref top, _) => top.absolute_ypoint(abs_parent),
                RectY::TopHeight(ref top, _) => top.absolute_ypoint(abs_parent),
                RectY::BottomHeight(ref bottom, ref height) =>
                    bottom.absolute_ypoint(abs_parent) - height.absolute_ydim(abs_parent),
            },
            width: match self.rectx {
                RectX::LeftRight(ref left, ref right) =>
                    right.absolute_xpoint(abs_parent) - left.absolute_xpoint(abs_parent),
                RectX::LeftWidth(_, ref width) => width.absolute_xdim(abs_parent),
                RectX::RightWidth(_, ref width) => width.absolute_xdim(abs_parent),
            },
            height: match self.recty {
                RectY::TopBottom(ref top, ref bottom) =>
                    bottom.absolute_ypoint(abs_parent) - top.absolute_ypoint(abs_parent),
                RectY::TopHeight(_, ref height) => height.absolute_ydim(abs_parent),
                RectY::BottomHeight(_, ref height) => height.absolute_ydim(abs_parent),
            }
        };

        // crop
        if absrect.x < abs_parent.x { absrect.x = abs_parent.x; }
        if absrect.x + absrect.width > abs_parent.x + abs_parent.width {
            absrect.width = abs_parent.x + abs_parent.width - absrect.x;
            // FIXME: ensure width is at least 1.0
        }
        if absrect.y < abs_parent.y { absrect.y = abs_parent.y; }
        if absrect.y + absrect.height > abs_parent.y + abs_parent.height {
            absrect.height = abs_parent.y + abs_parent.height - absrect.y;
            // FIXME: ensure height is at least 1.0
        }

        absrect
    }
}

impl Default for Rect {
    fn default() -> Rect {
        Rect {
            rectx: RectX::LeftWidth(Coord::near(0.0, 0), Dim::new(0.0, 0)),
            recty: RectY::TopHeight(Coord::near(0.0, 0), Dim::new(0.0, 0)),
        }
    }
}
