
use super::AbsRect;

/// Specify a dimensional size as a combination of a fraction of some
/// total size, and a pixel offset from that
#[derive(Debug, Clone, Copy)]
pub struct Dim {
    pub fraction: f32,
    pub pixel_offset: i32,
}
impl Dim {
    #[inline]
    pub fn new(fraction: f32, pixel_offset: i32) -> Dim {
        Dim {
            fraction: fraction,
            pixel_offset: pixel_offset
        }
    }

    #[inline]
    pub fn absolute_xdim(&self, abs_parent: &AbsRect) -> f32 {
        abs_parent.width * self.fraction + self.pixel_offset as f32
    }

    #[inline]
    pub fn absolute_ydim(&self, abs_parent: &AbsRect) -> f32 {
        abs_parent.height * self.fraction + self.pixel_offset as f32
    }
}
