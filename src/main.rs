
#![recursion_limit = "1024"]

// Include our macros early
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/macros.rs"));

// serialization
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate bincode;

// configuration
extern crate toml;

// errors
#[macro_use]
extern crate error_chain;

// logging
#[macro_use]
extern crate log;

// graphics
extern crate vks;
extern crate dacite;
extern crate dacite_winit;
extern crate winit;
extern crate siege_mesh;
#[macro_use]
extern crate siege_render;
extern crate siege_font;

// files
extern crate ddsfile;
extern crate zstd;

// win32
#[cfg(windows)] extern crate user32;
#[cfg(windows)] extern crate winapi;

// system types
extern crate libc;

// networking
extern crate mio;
extern crate siege_net;
extern crate siege_example_net;
extern crate ring;

// math
extern crate siege_math;

// time
extern crate chrono;

// data structures
extern crate bit_vec;
extern crate crossbeam;
extern crate chashmap;

// plugins
extern crate siege_plugin_avatar_simple;

mod errors;
pub use errors::*;

mod config;
use config::Config;

mod logger;

mod state;
use state::State;

mod input;

mod graphics;

mod network;

mod camera;

mod terrain;

mod ui;

mod stats;

mod chat;

// These maximums are due to the size of memory chunks that we define in
// graphics/memory.rs.  4K resolution is the maximum that we support.
const MAX_WIDTH: u32 = 3840;
const MAX_HEIGHT: u32 = 2160;

fn error_dump(e: &Error) {
    use std::io::Write;
    let stderr = &mut ::std::io::stderr();
    let errmsg = "Error writing to stderr";

    writeln!(stderr, "error: {}", e).expect(errmsg);

    for e in e.iter().skip(1) {
        writeln!(stderr, "caused by: {}", e).expect(errmsg);
    }

    if let Some(backtrace) = e.backtrace() {
        writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
    }
}


fn main() {
    if let Err(ref e) = run() {
        error_dump(e);

        #[cfg(windows)]
        {
            use std::io::Read;
            println!("Press a key to exit");
            let mut buffer: [u8; 1] = [0; 1];
            let _ = ::std::io::stdin().read_exact(&mut buffer);

        }
        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    use std::sync::Arc;
    use std::thread;
    use winit::{EventsLoop, WindowBuilder};

    // Load configuration
    let mut config = Config::load()
        .chain_err(|| "Unable to load configuration")?;
    // Set configuration settings that we know better
    config.graphics.renderer.app_name = "Siege Sample Client".to_owned();
    config.graphics.renderer.major_version = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
    config.graphics.renderer.minor_version = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap();
    config.graphics.renderer.patch_version = env!("CARGO_PKG_VERSION_PATCH").parse().unwrap();
    config.graphics.renderer.width = config.window.width;
    config.graphics.renderer.height = config.window.height;
    config.graphics.renderer.max_descriptor_sets = 13;
    config.graphics.renderer.max_uniform_buffers = 6;
    config.graphics.renderer.max_uniform_texel_buffers = 1;
    config.graphics.renderer.max_dynamic_uniform_buffers = 1;
    config.graphics.renderer.max_samplers = 1;
    config.graphics.renderer.max_sampled_images = 1;
    config.graphics.renderer.max_combined_image_samplers = 18;

    let arc_config = Arc::new(config.clone());

    // Start logging
    logger::init(&arc_config)?;

    info!("siege-example-client starting up.");

    info!("Config:\r\n{:?}", arc_config);
    trace!("Tracing is enabled.");

    // Create shared state
    let arc_state: Arc<State> = Arc::new(state::State::new(&arc_config)?);

    // Setup a custom panic hook (On any panic, we want to set the terminating bool)
    let default_panic_hook = ::std::panic::take_hook();
    let panichook_state = arc_state.clone();
    ::std::panic::set_hook(Box::new(move |panicinfo| {
        use std::sync::atomic::Ordering;
        panichook_state.terminating.store(true, Ordering::Relaxed);
        default_panic_hook(panicinfo);
    }));

    // Setup the network system
    let mut network_system = network::NetworkSystem::new(
        arc_state.clone(), arc_config.clone())?;
    let net_guard = thread::spawn(move|| {
        if let Err(ref e) = network_system.run() {
            error_dump(e);
        }
    });

    // Setup the winit event loop
    let events_loop = EventsLoop::new();

    // Setup the window
    let arc_window = {
        let mut builder = WindowBuilder::new()
            .with_title("Siege Sample Client")
            .with_visibility(false) // will be turned on when graphics are ready
            .with_transparency(false)
            .with_max_dimensions(MAX_WIDTH, MAX_HEIGHT);

        if arc_config.window.fullscreen {
            let maybe_screen = events_loop
                .get_available_monitors()
                .nth(arc_config.window.screen_number);
            builder = builder.with_fullscreen(maybe_screen)
                .with_decorations(false);
        } else {
            builder = builder
                .with_dimensions(arc_config.window.width, arc_config.window.height)
                .with_decorations(true);
        }

        let window = builder.build(&events_loop)?;

        Arc::new(window)
    };

    // Start graphics
    let graphics_config = arc_config.clone();
    let graphics_state = arc_state.clone();
    let graphics_state_extra = arc_state.clone();
    let graphics_window = arc_window.clone();
    let gfx_guard = thread::spawn(move|| {
        match graphics::GraphicsSystem::new(
            graphics_config, graphics_state, graphics_window)
        {
            Ok(mut gs) => if let Err(e) = gs.run() {
                // run() already signaled the other systems that we are terminating.
                error_dump(&From::from(e));
            },
            Err(e) => {
                use dacite::core::Error::InitializationFailed;
                if let &ErrorKind::Dacite(InitializationFailed) = e.kind() {
                    error_dump(&Error::with_chain(
                        e, "Vulkan initialization (of something) failed. This is fatal."));
                } else {
                    error_dump(&e)
                }

                // Could not start graphics, let other systems know we are terminating.
                use std::sync::atomic::Ordering;
                graphics_state_extra.terminating.store(true, Ordering::Relaxed);
            }
        }
    });

    // Throw up some UI windows
    {
        use ui::{UiElement, Coord};
        use ui::{TextLine, Point, TextColor, Font};
        let ui = &arc_state.ui;

        ui.add_element(UiElement::Text(TextLine {
            ui_coordinates: Point { x: Coord::near(0.0, 8), y: Coord::near(0.0, 28) },
            lineheight: 32,
            color: TextColor::Blue,
            outline: Some(TextColor::Gold),
            font: Font::Fantasy,
            alpha: 192,
            text: "Siege Sample Client".to_owned()
        }), None);
    }

    // Add some chat text
    {
        let mut chat = arc_state.chat.write().unwrap();
        let ui = &arc_state.ui;
        chat.emit_line(ui, "Welcome to the Siege Sample Client");
        chat.emit_line(ui, " [WIN]+[ESC] exits.");
        chat.emit_line(ui, " Use WASDQE keys to move. PgUp/PgDown tilts camera.");
        chat.emit_line(ui, " F3/F4 zoom  F7/F8 blur");
        chat.emit_line(ui, " F9/F10 bloom strength, F11/F12 bloom cliff");
    }

    trace!("All systems go. Main thread waiting for child threads to complete.");

    println!("Press and release <LOGO>-Esc to quit (or close the window)");

    // Start handing input
    let mut input_system = input::InputSystem::new(
        arc_state.clone(), arc_window.clone());
    if let Err(ref e) = input_system.run(events_loop) {
        error_dump(e);
    }

    // Wait for the graphics system to end
    let _ = gfx_guard.join();
    trace!("Graphics system thread has completed.");

    // Wait for the network system to end
    let _ = net_guard.join();
    trace!("Network system thread has completed.");

    Ok(())
}
