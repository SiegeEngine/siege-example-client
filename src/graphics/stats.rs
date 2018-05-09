
use std::sync::Arc;
use std::time::Instant;
use errors::*;
use dacite::core::{CommandBuffer, Extent2D};
use siege_render::{Params, Plugin};
use siege_render::Stats as RenderStats;
use ui::{Handle, TextLine, Point, Dim, Coord, Font, TextColor, UiElement, UiWindow,
         Rect, RectX, RectY};
use State;

fn maketext(state: &State, line: TextLine, parent: Option<Handle>) -> Handle {
    let handle = state.ui.add_element(UiElement::Text(line), parent).unwrap();
    handle
}

pub struct StatsGfx {
    state: Arc<State>,
    render_stats_last_updated: Instant,

    fps_line: Handle,
    geometry_line: Handle,
    shading_line: Handle,
    transparent_line: Handle,
    blur1_line: Handle,
    blur2_line: Handle,
    post_line: Handle,
    ui_line: Handle,
    overhead_line: Handle,

    render_line: Handle,
    cpu_line: Handle,
    frame_line: Handle,

    clocksync_line: Handle,

    #[allow(dead_code)]
    win_handle: Handle,
    #[allow(dead_code)]
    win: UiWindow,
}

impl StatsGfx {
    pub fn new(state: Arc<State>) -> Result<StatsGfx>
    {
        let win = UiWindow::new(
            Rect::new(RectX::RightWidth(Coord::far(0.0, -5), Dim::new(0.0, 167)),
                      RectY::TopHeight(Coord::near(0.0, 5), Dim::new(0.0, 255))),
            [0.0, 0.0, 0.0, 0.5], 0.8
        );
        let win_handle = state.ui.add_element(UiElement::Window(win.clone()), None).unwrap();

        const LINEHEIGHT: u8 = 14;
        let mut y: i32 = 22;

        let fps_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::Green,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let geometry_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::White,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let shading_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::White,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let transparent_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::White,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let blur1_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::White,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let blur2_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::White,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let post_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::White,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let ui_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::White,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let overhead_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::White,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let render_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::Green,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let cpu_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::Green,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32;
        let frame_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::Green,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        y+=LINEHEIGHT as i32 *3 /2;
        let clocksync_line = maketext(&state, TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 10), y: Coord::near(0.0, y) },
            lineheight: LINEHEIGHT,
            color: TextColor::Gold,
            outline: None,
            font: Font::Mono,
            alpha: 255,
            text: " ".to_owned()
        }, Some(win_handle));

        Ok(StatsGfx {
            state: state,
            render_stats_last_updated: Instant::now(),
            fps_line: fps_line,
            geometry_line: geometry_line,
            shading_line: shading_line,
            transparent_line: transparent_line,
            blur1_line: blur1_line,
            blur2_line: blur2_line,
            post_line: post_line,
            ui_line: ui_line,
            overhead_line: overhead_line,
            render_line: render_line,
            cpu_line: cpu_line,
            frame_line: frame_line,
            clocksync_line: clocksync_line,
            win_handle: win_handle,
            win: win,
        })
    }
}

impl Plugin for StatsGfx {
    fn record_geometry(&self, _command_buffer: CommandBuffer) {
    }

    fn record_transparent(&self, _command_buffer: CommandBuffer) {
    }

    fn record_ui(&self, _command_buffer: CommandBuffer) {
    }

    fn update(&mut self, _params: &mut Params, renderstats: &RenderStats)
              -> ::siege_render::Result<bool>
    {
        // Only update if renderstats were updated (typically every 60 frames)
        // (this means non-renderstats also are updated only periodically)
        if renderstats.last_updated == self.render_stats_last_updated {
            return Ok(false);
        }
        self.render_stats_last_updated = renderstats.last_updated;

        // Render stats
        {
            let ui = &self.state.ui;
            ui.set_text(self.fps_line,
                        format!("FPS: {:4.1}",
                                60_000.0 / renderstats.timings_60.frame));
            ui.set_text(self.geometry_line,
                        format!("    geometry: {:05.2}",
                                renderstats.timings_60.geometry / 60.0));
            ui.set_text(self.shading_line,
                        format!("    shading:  {:05.2}",
                                renderstats.timings_60.shading / 60.0));
            ui.set_text(self.transparent_line,
                        format!("    trans:    {:05.2}",
                                renderstats.timings_60.transparent / 60.0));
            ui.set_text(self.blur1_line,
                        format!("    blur1:    {:05.2}",
                                renderstats.timings_60.blur1 / 60.0));
            ui.set_text(self.blur2_line,
                        format!("    blur2:    {:05.2}",
                                renderstats.timings_60.blur2 / 60.0));
            ui.set_text(self.post_line,
                        format!("    post:     {:05.2}",
                                renderstats.timings_60.post / 60.0));
            ui.set_text(self.ui_line,
                        format!("    ui:       {:05.2}",
                                renderstats.timings_60.ui / 60.0));
            ui.set_text(self.overhead_line,
                        format!("    over:     {:05.2}",
                                (renderstats.timings_60.render - (
                                    renderstats.timings_60.geometry +
                                        renderstats.timings_60.shading +
                                        renderstats.timings_60.transparent +
                                        renderstats.timings_60.blur1 +
                                        renderstats.timings_60.blur2 +
                                        renderstats.timings_60.post +
                                        renderstats.timings_60.ui
                                )) / 60.0));
            ui.set_text(self.render_line,
                        format!("  ||render:   {:05.2}",
                                renderstats.timings_60.render / 60.0));
            ui.set_text(self.cpu_line,
                        format!("  ||cpu:      {:05.2}",
                                renderstats.timings_60.cpu / 60.0));
            ui.set_text(self.frame_line,
                        format!("frame:        {:05.2}",
                                renderstats.timings_60.frame / 60.0));

            let stats = self.state.stats.read().unwrap();
            ui.set_text(self.clocksync_line,
                        format!("svr sync:   {}ms",  stats.network_clocksync_ms));
        }

        Ok(false)
    }

    fn gpu_update(&mut self) -> ::siege_render::Result<()> {
        Ok(())
    }

    fn rebuild(&mut self, _extent: Extent2D) -> ::siege_render::Result<()> {
        Ok(())
    }
}

