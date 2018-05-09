
use errors::*;
use log::{Record, Level, Metadata};
use config::Config;

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum CodeLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl CodeLogLevel {
    pub fn as_log_level(&self) -> Level {
        match *self {
            CodeLogLevel::Error => Level::Error,
            CodeLogLevel::Warn => Level::Warn,
            CodeLogLevel::Info => Level::Info,
            CodeLogLevel::Debug => Level::Debug,
            CodeLogLevel::Trace => Level::Trace,
        }
    }
}

struct SiegeLogger {
    log_level: Level,
    log_fileline: bool,
}

pub fn init(config: &Config) -> Result<()> {
    let log_level = config.code_log_level.as_log_level();

    ::log::set_boxed_logger(
        Box::new(SiegeLogger {
            log_level: log_level,
            log_fileline: config.code_log_fileline,
        })
    )?;
    ::log::set_max_level(log_level.to_level_filter());
    Ok(())
}

impl ::log::Log for SiegeLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.log_level
            && metadata.target().starts_with("siege")
    }

    fn log(&self, record: &Record) {
        use std::io::Write;

        // Only log relevant stuff
        if !self.enabled(record.metadata()) {
            return;
        }

        let stderr = &mut ::std::io::stderr();
        let badstderr = "Error writing to stderr";

        if self.log_fileline {
            let mp = record.module_path().unwrap_or("?");
            let f = record.file().unwrap_or("?");
            let l = match record.line() {
                Some(l) => format!("{}",l),
                None => "?".to_owned(),
            };
            write!(stderr, "{} [{}:{}] ", mp, f, l)
                .expect(badstderr);
        }

        // record.target() is currently not used.  If not set in a log macro,
        // it ends up being the module path.  In the future we could use it to
        // indicate if logging should go to a the console or to the user via
        // the graphics.

        writeln!(stderr, "{}: {}", record.level(), record.args())
            .expect(badstderr);
    }

    fn flush(&self) { }
}
