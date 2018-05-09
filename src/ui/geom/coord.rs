
use super::{AbsRect, Dim};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopOrLeft,
    BottomOrRight,
}

/// Specify a point coordinate, as a combination of an anchor
/// and a dimensional size away from the anchor
#[derive(Debug, Clone, Copy)]
pub struct Coord {
    pub anchor: Anchor,
    pub dim: Dim,
}
impl Coord {
    #[inline]
    pub fn near(fraction: f32, pixel_offset: i32) -> Coord {
        Coord {
            anchor: Anchor::TopOrLeft,
            dim: Dim {
                fraction: fraction,
                pixel_offset: pixel_offset
            }
        }
    }
    #[inline]
    pub fn far(fraction: f32, pixel_offset: i32) -> Coord {
        Coord {
            anchor: Anchor::BottomOrRight,
            dim: Dim {
                fraction: fraction,
                pixel_offset: pixel_offset
            }
        }
    }

    pub fn absolute_xpoint(&self, abs_parent: &AbsRect) -> f32 {
        let anch = if self.anchor == Anchor::BottomOrRight {
            abs_parent.width
        } else {
            0.0
        };
        let frac = self.dim.fraction * abs_parent.width;
        let px = self.dim.pixel_offset as f32;
        abs_parent.x + anch + frac + px
    }
    pub fn absolute_ypoint(&self, abs_parent: &AbsRect) -> f32 {
        let anch = if self.anchor == Anchor::BottomOrRight {
            abs_parent.height
        } else {
            0.0
        };
        let frac = self.dim.fraction * abs_parent.height;
        let px = self.dim.pixel_offset as f32;
        abs_parent.y + anch + frac + px
    }
}
