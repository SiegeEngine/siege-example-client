
use super::UiWindow;
use ui::{Ui, Coord, Dim, Rect, RectX, RectY, UiElement, Handle, UiImage, Anchor};
use ui::{WINDOW_TL_CORNER, WINDOW_TR_CORNER, WINDOW_TOP,
         WINDOW_BL_CORNER, WINDOW_BR_CORNER, WINDOW_BOTTOM,
         WINDOW_LEFT, WINDOW_RIGHT, WINDOW_PANE};

impl UiWindow {
    // Decorates window that handle refers to, and returns a Handle to an inner
    // window, inside of the decoration area.
    pub fn decorate_window(ui: &Ui,
                           handle: Handle,
                           mut _splits: Vec<Coord>,
                           title: Option<&str>)
                           -> Handle
    {
        let border_width = Dim::new(0.0, WINDOW_LEFT.width as i32);
        let border_height = Dim::new(0.0, WINDOW_TOP.height as i32);

        let outer_left = coord_near!(0.0, 0);
        let outer_right = coord_far!(0.0, 0);
        let inner_left = coord_near!(0.0, WINDOW_LEFT.width as i32);
        let inner_right = coord_far!(0.0, -WINDOW_RIGHT.width as i32);

        let outer_top = coord_near!(0.0, 0);
        let inner_top = coord_near!(0.0, WINDOW_TOP.height as i32);
        let inner_bottom = coord_far!(0.0, -WINDOW_BOTTOM.height as i32);
        let outer_bottom = coord_far!(0.0, 0);

        // top --------------------------------------

        let r = Rect {
            rectx: RectX::LeftWidth(outer_left, border_width),
            recty: RectY::TopHeight(outer_top, border_height)
        };
        let _ = ui.add_element(UiElement::Image(UiImage {
            widget: WINDOW_TL_CORNER,
            widget_pin_rect: r.clone(),
            screen_draw_rect: r.clone(),
        }), Some(handle)).unwrap();

        let r = Rect {
            rectx: RectX::RightWidth(outer_right, border_width),
            recty: RectY::TopHeight(outer_top, border_height)
        };
        let _ = ui.add_element(UiElement::Image(UiImage {
            widget: WINDOW_TR_CORNER,
            widget_pin_rect: r.clone(),
            screen_draw_rect: r.clone(),
        }), Some(handle)).unwrap();

        let _ = ui.add_element(UiElement::Image(UiImage {
            widget: WINDOW_TOP,
            widget_pin_rect: Rect {
                rectx: RectX::LeftWidth(inner_left, border_width),
                recty: RectY::TopHeight(outer_top, border_height),
            },
            screen_draw_rect: Rect {
                rectx: RectX::LeftRight(inner_left, inner_right),
                recty: RectY::TopHeight(outer_top, border_height),
            },
        }), Some(handle)).unwrap();

        // title --------------------------------------

        // Left
        let _ = ui.add_element(UiElement::Image(UiImage {
            widget: WINDOW_LEFT,
            widget_pin_rect: Rect {
                rectx: RectX::LeftWidth(outer_left, border_width),
                recty: RectY::TopHeight(inner_top, border_height)
            },
            screen_draw_rect: Rect {
                rectx: RectX::LeftWidth(outer_left, border_width),
                recty: RectY::TopBottom(inner_top, inner_bottom)
            },
        }), Some(handle)).unwrap();

        // Right
        let _ = ui.add_element(UiElement::Image(UiImage {
            widget: WINDOW_RIGHT,
            widget_pin_rect: Rect {
                rectx: RectX::RightWidth(outer_right, border_width),
                recty: RectY::TopHeight(inner_top, border_height)
            },
            screen_draw_rect: Rect {
                rectx: RectX::RightWidth(outer_right, border_width),
                recty: RectY::TopBottom(inner_top, inner_bottom)
            },
        }), Some(handle)).unwrap();

        // Pane
        let _ = ui.add_element(UiElement::Image(UiImage {
            widget: WINDOW_PANE,
            widget_pin_rect: Rect {
                rectx: RectX::LeftWidth(inner_left, border_width),
                recty: RectY::TopHeight(inner_top, border_height),
            },
            screen_draw_rect: Rect {
                rectx: RectX::LeftRight(inner_left, inner_right),
                recty: RectY::TopBottom(inner_top, inner_bottom),
            },
        }), Some(handle)).unwrap();


        // title split --------------------------------

        // main section -------------------------------

        // bottom -------------------------------------


        // BL corner
        let r = Rect {
            rectx: RectX::LeftWidth(outer_left, border_width),
            recty: RectY::BottomHeight(outer_bottom, border_height),
        };
        let _ = ui.add_element(UiElement::Image(UiImage {
            widget: WINDOW_BL_CORNER,
            widget_pin_rect: r.clone(),
            screen_draw_rect: r.clone(),
        }), Some(handle)).unwrap();

        // BR corner
        let r = Rect {
            rectx: RectX::RightWidth(outer_right, border_width),
            recty: RectY::BottomHeight(outer_bottom, border_height)
        };
        let _ = ui.add_element(UiElement::Image(UiImage {
            widget: WINDOW_BR_CORNER,
            widget_pin_rect: r.clone(),
            screen_draw_rect: r.clone(),
        }), Some(handle)).unwrap();

        // Bottom
        let _ = ui.add_element(UiElement::Image(UiImage {
            widget: WINDOW_BOTTOM,
            widget_pin_rect: Rect {
                rectx: RectX::LeftWidth(inner_left, border_width),
                recty: RectY::BottomHeight(outer_bottom, border_height)
            },
            screen_draw_rect: Rect {
                rectx: RectX::LeftRight(inner_left, inner_right),
                recty: RectY::BottomHeight(outer_bottom, border_height)
            },
        }), Some(handle)).unwrap();

        if let Some(title) = title {
            // Title: FIXME, return handle to this, they may want to change it
            //     ALSO allow them to set the details (color, etc).
            use ui::{TextColor, Font, TextLine, Point};
            let _ = ui.add_element(UiElement::Text(TextLine {
                ui_coordinates: Point {
                    x: coord_near!(0.0, WINDOW_LEFT.width as i32 + 5),
                    y: coord_near!(0.0, WINDOW_TOP.height as i32 - 2),
                },
                lineheight: WINDOW_TOP.height as u8 - 4,
                color: TextColor::Black,
                outline: None,
                font: Font::Mono,
                alpha: 255,
                text: title.to_owned()
            }), Some(handle)).unwrap();
        }

        // Inner Window
        ui.add_element(
            UiElement::Window(UiWindow::new(
                Rect {
                    rectx: RectX::LeftRight(inner_left, inner_right),
                    recty: RectY::TopBottom(inner_top, inner_bottom)
                },
                [0.0, 0.0, 0.0, 0.0], 1.0)
            ),
            Some(handle)).unwrap()
    }
}
