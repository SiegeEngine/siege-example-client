
use errors::*;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use state::State;
use winit::{Window, EventsLoop, Event, WindowEvent, WindowId, KeyboardInput,
            DeviceId, ModifiersState};
use siege_net::packets::ShutdownPacket;
use siege_example_net::packet::GamePacket;
use siege_math::Angle;
use siege_plugin_avatar_simple::MoveDirection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Chat,
    Command,
}

pub struct InputSystem {
    mode: Mode,
    state: Arc<State>,
    #[allow(dead_code)]
    window: Arc<Window>
}

impl InputSystem {
    pub fn new(state: Arc<State>, window: Arc<Window>)
               -> InputSystem
    {
        InputSystem {
            mode: Mode::Normal,
            state: state,
            window: window,
        }
    }

    pub fn run(&mut self, event_loop: EventsLoop)
               -> Result<()>
    {
        let result = self._run(event_loop);
        if result.is_err() {
            self.state.terminating.store(true, Ordering::Relaxed);
        }
        result
    }

    pub fn _run(&mut self, mut event_loop: EventsLoop)
               -> Result<()>
    {
        use std::time::{Duration, Instant};

        // 60 events per second, max
        let loop_throttle = Duration::new(0, 1_000_000_000 / 60);
        let mut loop_start: Instant;

        loop {
            loop_start = Instant::now();

            // Handle events
            event_loop.poll_events(|e: Event| self.handle_event(e));

            // Shutdown when it is time to do so
            if self.state.terminating.load(Ordering::Relaxed) {

                // Tell our buddy the network system, since he doesn't check this
                // atomic boolean
                self.state.packet_sender.send(GamePacket::Shutdown(ShutdownPacket::new()))?;

                trace!("Input (window) system loop has completed.");
                return Ok(());
            }

            // Update state
            self.state.periodic_update();

            // Throttle
            let loop_duration = Instant::now().duration_since(loop_start);
            if loop_duration < loop_throttle {
                ::std::thread::sleep(loop_throttle - loop_duration);
            }
        }
    }

    pub fn handle_event(&mut self, e: Event)
    {
        match e {
            Event::WindowEvent { window_id, event } =>
                self.handle_window_event(event, window_id),
            Event::DeviceEvent { .. } => {},
            Event::Awakened => {},
            Event::Suspended(_) => {},
        }
    }

    pub fn handle_window_event(&mut self, e: WindowEvent, _window_id: WindowId)
    {
        match e {
            WindowEvent::Resized(_,_) => {
                // This affects vulkan;  We record this event so we can rebuild the
                // swapchain.
                self.state.resized.store(true, Ordering::Relaxed);
            },
            WindowEvent::Closed => {
                // This starts the shutdown sequence
                self.state.terminating.store(true, Ordering::Relaxed);
            },
            WindowEvent::ReceivedCharacter(ch) => {
                // This handles characters, which is very useful for unicode input,
                // rather than key up/down events.  We use it for chat/command modes.
                if self.mode == Mode::Chat || self.mode == Mode::Command {
                    self.handle_character(ch);
                }
            },
            WindowEvent::KeyboardInput { device_id, input } => {
                // This handles up/down events.  We use it for normal mode.
                if self.mode == Mode::Normal {
                    self.handle_keyboard(device_id, input);
                }
            }
            WindowEvent::CursorMoved { device_id, position, modifiers } => {
                // x and y are pixel positions within the window
                self.handle_mouse(device_id, position.0, position.1, modifiers);
            }
            _ => { }
        }
    }

    pub fn handle_character(&mut self, ch: char)
    {
        if ch=='\u{001b}' { // Escape
            self.mode = Mode::Normal;
            // ignore command/chat buffers.
        }
        else if ch=='\u{000d}' { // CR (Enter)
            self.mode = Mode::Normal;
            // FIXME: handle command or chat buffer
        }
        else {
            debug!("Character: {} = {}", ch, ch.escape_unicode());
        }
    }

    pub fn handle_keyboard(&mut self, _device_id: DeviceId, input: KeyboardInput)
    {
        use winit::ElementState;
        use winit::VirtualKeyCode as Key;

        // We only handle keys with virtual keycodes (currently)
        // (NumLock, PrintScreen, ScrollLock and Pause do not have codes, but are often
        //  used by the O.S. and we really ought not overload their meaning)
        let key = match input.virtual_keycode {
            None => return,
            Some(k) => k
        };

        // We trigger some special keys on release
        if let ElementState::Released = input.state {
            match (input.modifiers.shift, input.modifiers.ctrl, input.modifiers.alt,
                   input.modifiers.logo, key)
            {
                // "LOGO-ESCAPE" quits, when the key is released.
                (_,_,_,true,Key::Escape) => {
                    // This starts the shutdown sequence
                    self.state.terminating.store(true, Ordering::Relaxed);
                },
                (_,false,false,false,Key::W) | (_,false,false,false,Key::Up) =>
                    self.state.movement_cmd(MoveDirection::Forward, false),
                (_,false,false,false,Key::S) | (_,false,false,false,Key::Down) =>
                    self.state.movement_cmd(MoveDirection::Backward, false),
                (_,false,false,false,Key::A) | (_,false,false,false,Key::Left) =>
                    self.state.movement_cmd(MoveDirection::YawLeft, false),
                (_,false,false,false,Key::D) | (_,false,false,false,Key::Right) =>
                    self.state.movement_cmd(MoveDirection::YawRight, false),
                (_,false,false,false,Key::Q) =>
                    self.state.movement_cmd(MoveDirection::StrafeLeft, false),
                (_,false,false,false,Key::E) =>
                    self.state.movement_cmd(MoveDirection::StrafeRight, false),
                (_,false,false,false,Key::PageUp) =>
                    self.state.movement_cmd(MoveDirection::PitchUp, false),
                (_,false,false,false,Key::PageDown) =>
                    self.state.movement_cmd(MoveDirection::PitchDown, false),
                _ => {}
            }
        } else {
            match (input.modifiers.shift, input.modifiers.ctrl, input.modifiers.alt,
                   input.modifiers.logo, key)
            {
                (_,false,false,false,Key::Grave) => self.mode = Mode::Chat,
                (_,false,false,false,Key::Slash) => self.mode = Mode::Command,
                (_,false,false,false,Key::W) | (_,false,false,false,Key::Up) =>
                    self.state.movement_cmd(MoveDirection::Forward, true),
                (_,false,false,false,Key::S) | (_,false,false,false,Key::Down) =>
                    self.state.movement_cmd(MoveDirection::Backward, true),
                (_,false,false,false,Key::A) | (_,false,false,false,Key::Left) =>
                    self.state.movement_cmd(MoveDirection::YawLeft, true),
                (_,false,false,false,Key::D) | (_,false,false,false,Key::Right) =>
                    self.state.movement_cmd(MoveDirection::YawRight, true),
                (_,false,false,false,Key::Q) =>
                    self.state.movement_cmd(MoveDirection::StrafeLeft, true),
                (_,false,false,false,Key::E) =>
                    self.state.movement_cmd(MoveDirection::StrafeRight, true),
                (_,false,false,false,Key::PageUp) =>
                    self.state.movement_cmd(MoveDirection::PitchUp, true),
                (_,false,false,false,Key::PageDown) =>
                    self.state.movement_cmd(MoveDirection::PitchDown, true),
                (_,false,false,false,Key::F3) =>
                    self.state.adjust_fovx(Angle::<f32>::from_degrees(0.5)),
                (_,false,false,false,Key::F4) =>
                    self.state.adjust_fovx(Angle::<f32>::from_degrees(-0.5)),
                (_,false,false,false,Key::F7) =>
                    self.state.adjust_blur_level(false),
                (_,false,false,false,Key::F8) =>
                    self.state.adjust_blur_level(true),
                (_,false,false,false,Key::F9) =>
                    self.state.adjust_bloom_strength(false),
                (_,false,false,false,Key::F10) =>
                    self.state.adjust_bloom_strength(true),
                (_,false,false,false,Key::F11) =>
                    self.state.adjust_bloom_cliff(false),
                (_,false,false,false,Key::F12) =>
                    self.state.adjust_bloom_cliff(true),
                (_,_,_,_,key) => trace!("KEY: {:?}", key),
            }
        }
    }

    pub fn handle_mouse(&self, _device_id: DeviceId, _x: f64, _y: f64,
                        _modifers: ModifiersState)
    {
    }
}
