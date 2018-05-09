
use errors::*;
use std::env;
use std::default::Default;
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::str::FromStr;
use std::fmt;
use toml;

use logger::CodeLogLevel;
use siege_render::Config as RenderConfig;

//--------------------------------------------------

#[inline] fn default_render_config() -> RenderConfig {
    Default::default()
}

#[derive(Clone, Deserialize)]
pub struct GraphicsConfig {
    #[serde(default = "default_render_config")]
    pub renderer: RenderConfig,
}

impl Default for GraphicsConfig {
    fn default() -> GraphicsConfig {
        GraphicsConfig {
            renderer: Default::default(),
        }
    }
}

impl fmt::Debug for GraphicsConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "  renderer:")?;
        write!(f, "{:?}", self.renderer)?;
        Ok(())
    }
}

//--------------------------------------------------


#[inline] fn default_server_public_key() -> [u8; 32] {
    [0, 0, 0, 0, 0, 0, 0, 0,
     0, 0, 0, 0, 0, 0, 0, 0,
     0, 0, 0, 0, 0, 0, 0, 0,
     0, 0, 0, 0, 0, 0, 0, 0]
}
#[inline] fn default_server_socket_addr() -> SocketAddr {
    FromStr::from_str("127.0.0.1:5555").unwrap()
}
#[inline] fn default_connect_on_startup() -> bool { false }

#[derive(Clone, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_server_public_key")]
    pub server_public_key: [u8; 32],
    #[serde(default = "default_server_socket_addr")]
    pub server_socket_addr: SocketAddr,
    #[serde(default = "default_connect_on_startup")]
    pub connect_on_startup: bool,
}

impl Default for NetworkConfig {
    fn default() -> NetworkConfig {
        NetworkConfig {
            server_public_key: default_server_public_key(),
            server_socket_addr: default_server_socket_addr(),
            connect_on_startup: default_connect_on_startup(),
        }
    }
}

impl fmt::Debug for NetworkConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "    server public key: [ELIDED]")?;
        writeln!(f, "    server socket addr: {}", self.server_socket_addr)?;
        writeln!(f, "    connect on startup: {}", self.connect_on_startup)?;
        Ok(())
    }
}

//--------------------------------------------------

#[inline] fn default_fullscreen() -> bool { false }
#[inline] fn default_width() -> u32 { 1280 }
#[inline] fn default_height() -> u32 { 720 }
#[inline] fn default_screen_number() -> usize { 0 }

#[derive(Clone, Deserialize)]
pub struct WindowConfig {
    #[serde(default = "default_fullscreen")]
    pub fullscreen: bool,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_screen_number")]
    pub screen_number: usize,
}

impl Default for WindowConfig {
    fn default() -> WindowConfig {
        WindowConfig {
            fullscreen: default_fullscreen(),
            screen_number: default_screen_number(),
            width: default_width(),
            height: default_height(),
        }
    }
}

impl fmt::Debug for WindowConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "    fullscreen: {}", self.fullscreen)?;
        if ! self.fullscreen {
            writeln!(f, "    width: {}", self.width)?;
            writeln!(f, "    height: {}", self.height)?;
        }
        writeln!(f, "    screen_number: {}", self.screen_number)?;
        Ok(())
    }
}

//--------------------------------------------------

#[inline] fn default_code_log_level() -> CodeLogLevel {
    if cfg!(debug_assertions) { CodeLogLevel::Debug }
    else { CodeLogLevel::Warn }
}
#[inline] fn default_code_log_fileline() -> bool { true }
#[inline] fn default_code_log_detailed_errors() -> bool { false }

#[derive(Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_code_log_level")]
    pub code_log_level: CodeLogLevel,
    #[serde(default = "default_code_log_fileline")]
    pub code_log_fileline: bool,
    #[serde(default = "default_code_log_detailed_errors")]
    pub code_log_detailed_errors: bool,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub graphics: GraphicsConfig,
    #[serde(default)]
    pub network: NetworkConfig,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            code_log_level: default_code_log_level(),
            code_log_fileline: default_code_log_fileline(),
            code_log_detailed_errors: default_code_log_detailed_errors(),
            window: Default::default(),
            graphics: Default::default(),
            network: Default::default(),
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "  code_log_level: {:?}", self.code_log_level)?;
        writeln!(f, "  code_log_fileline: {}", self.code_log_fileline)?;
        writeln!(f, "  code_log_detailed_errors: {}", self.code_log_detailed_errors)?;
        writeln!(f, "  window:")?;
        write!(f, "{:?}", self.window)?;
        writeln!(f, "  graphics:")?;
        write!(f, "{:?}", self.graphics)?;
        writeln!(f, "  network:")?;
        write!(f, "{:?}", self.network)?;
        Ok(())
    }
}

impl Config {
    // Get the path to the configuraiton file
    fn get_path() -> PathBuf {
        // Try first argument (this is supposed to be the config file)
        let args: Vec<String> = env::args().map(|e| e.to_owned()).collect();
        if args.len() >= 2 {
            return PathBuf::from(&args[1]);
        }

        // Try environment variable
        if let Ok(p) = env::var("SIEGE_CONFIG_FILE") {
            return PathBuf::from(p);
        }

        // Otherwise, look in the current directory for eob.toml
        PathBuf::from("./siege.toml")
    }

    pub fn load() -> Result<Config>
    {
        let path = Config::get_path();

        if ! path.is_file() {
            // Just use default config
            return Ok(Default::default());
        }

        let config = Config::from_file( path )?;

        Ok(config)
    }

    fn from_file(path: PathBuf) -> Result<Config>
    {
        let mut contents: String = String::new();
        let mut file = File::open(&path)?;
        file.read_to_string(&mut contents)?;
        Ok(toml::from_str(&*contents)?)
    }
}
