
use ui::{Ui, UiWindow, TextLine, Point, TextColor, Font, Dim, Coord,
         Rect, RectX, RectY, UiElement, Handle};

/// FIXME: this is a text-scrolling terminal, often used for chat windows,
/// but does not implement any 'chat' service - so rename it.
pub struct Chat {
    pub next: usize,
    pub handles: [Handle; 10],
    pub inner_win_handle: Handle,
    #[allow(dead_code)]
    pub win_handle: Handle,
    pub win: UiWindow,
}

impl Chat {
    pub fn new(ui: &Ui) -> Chat
    {
        let margin = 6;
        let win = UiWindow::new(
            Rect::new(RectX::LeftWidth(Coord::near(0.0, margin), Dim::new(0.0, 470)),
                      RectY::BottomHeight(Coord::far(0.0, -margin), Dim::new(0.0, 150))),
            [0.0, 0.0, 0.0, 0.0], 0.8
        );
        let win_handle = ui.add_element(UiElement::Window(win.clone()), None).unwrap();

        let inner_win_handle = UiWindow::decorate_window(
            ui, win_handle, vec![], Some("Chat"));

        let tl = TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 0), y: Coord::far(0.0, 0) },
            lineheight: 15,
            color: TextColor::White,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: "".to_owned()
        };

        let mut chat = Chat {
            next: 9,
            handles: [ Handle(0), Handle(0), Handle(0), Handle(0), Handle(0),
                       Handle(0), Handle(0), Handle(0), Handle(0), Handle(0) ],
            inner_win_handle: inner_win_handle,
            win_handle: win_handle,
            win: win,
        };

        // send to ui and get handles
        for slot in 0..10 {
            chat.handles[slot] = ui.add_element(UiElement::Text(tl.clone()),
                                                Some(chat.inner_win_handle)).unwrap();
        }

        // fix line positions
        chat.scroll(ui);

        chat
    }

    pub fn emit_line<'a>(&mut self, ui: &Ui, text: &'a str) {
        self.scroll(ui);
        let n = (self.next - 1) %10;
        ui.set_text(self.handles[n], text.to_owned());
    }

    fn scroll(&mut self, ui: &Ui) {
        const LINEDROP: i32 = 5; // just a guess really

        for slot in 0..10 {
            let distance_up = (self.next + 10 - slot) % 10;

            ui.upsert(
                self.handles[slot],
                || { unimplemented!() }, // shouldnt happen
                |n: &mut UiElement| {
                    if let &mut UiElement::Text(ref mut tl) = n {
                        tl.ui_coordinates.y =
                            Coord::far(0.0, -LINEDROP - (tl.lineheight as i32 * distance_up as i32));
                    }
                }
            );
        }
        self.next += 1;
    }
}
