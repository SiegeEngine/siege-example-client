use super::{Coord, AbsRect};

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: Coord,
    pub y: Coord,
}

impl Point {
    pub fn absolute(&self, abs_parent: &AbsRect) -> (f32,f32) {
        let x = self.x.absolute_xpoint(abs_parent);
        let y = self.y.absolute_ypoint(abs_parent);
        (x,y)
    }
}
