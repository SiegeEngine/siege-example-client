
macro_rules! coord_near (
    ($fraction:expr, $offset:expr) => (
        Coord {
            anchor: Anchor::TopOrLeft,
            dim: Dim {
                fraction: $fraction,
                pixel_offset: $offset
            }
        }
    );
);

#[allow(unused_macros)]
macro_rules! coord_far (
    ($fraction:expr, $offset:expr) => (
        Coord {
            anchor: Anchor::BottomOrRight,
            dim: Dim {
                fraction: $fraction,
                pixel_offset: $offset
            }
        }
    );
);
