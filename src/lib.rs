use humantime;
use log::*;
use serde_json::json;
use std::thread;
use std::time::SystemTime;

pub use log::Level;

pub enum FiaasEnv {
    Local,
    Dev,
    Prod,
}

struct FiaasLogger {
    finn_app: &'static str,
    env: FiaasEnv,
    level: log::Level,
}

impl Log for FiaasLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = humantime::format_rfc3339_millis(SystemTime::now());

            match self.env {
                FiaasEnv::Local => match record.level() {
                    Level::Error => eprintln!(
                        "[{timestamp} {level} {logger}] {message}",
                        timestamp = timestamp,
                        logger = record.target(),
                        level = record.level(),
                        message = record.args(),
                    ),
                    _ => println!(
                        "[{timestamp} {level} {logger}] {message}",
                        timestamp = timestamp,
                        logger = record.target(),
                        level = record.level(),
                        message = record.args(),
                    ),
                },
                FiaasEnv::Dev | FiaasEnv::Prod => {
                    let t = thread::current();
                    match record.level() {
                        Level::Error => eprintln!(
                            "{}",
                            serde_json::to_string(&json!({
                              "@version":1,
                              "@timestamp": timestamp.to_string(),
                              "logger": record.target(),
                              "thread": format!("{}-{:?}", t.name().unwrap_or("unnamed"), t.id()),
                              "level": record.level().to_string(),
                              "message": record.args(),
                              "finn_app": self.finn_app
                            }))
                            .unwrap()
                        ),
                        _ => println!(
                            "{}",
                            serde_json::to_string(&json!({
                              "@version":1,
                              "@timestamp": timestamp.to_string(),
                              "logger": record.target(),
                              "thread": format!("{}-{:?}", t.name().unwrap_or("unnamed"), t.id()),
                              "level": record.level().to_string(),
                              "message": record.args(),
                              "finn_app": self.finn_app
                            }))
                            .unwrap()
                        ),
                    }
                }
            }
        }
    }

    fn flush(&self) {}
}

pub fn try_init(
    finn_app: &'static str,
    env: FiaasEnv,
    level: Level,
) -> Result<(), log::SetLoggerError> {
    let r = log::set_boxed_logger(Box::new(FiaasLogger {
        finn_app,
        env,
        level,
    }));
    if r.is_ok() {
        log::set_max_level(level.to_level_filter());
    }
    r
}

pub fn init(finn_app: &'static str, env: FiaasEnv, level: Level) {
    try_init(finn_app, env, level).unwrap();
}

pub fn init_env(finn_app: &'static str) {
    let level = match std::env::var("RUST_LOG")
        .expect("RUST_LOG must be set")
        .as_ref()
    {
        "error" => Level::Error,
        "warn" => Level::Warn,
        "info" => Level::Info,
        "debug" => Level::Debug,
        "trace" => Level::Trace,
        _ => panic!("RUST_LOG must be one of error, warn, info, debug and trace"),
    };

    let env = match std::env::var("FIAAS_ENV")
        .expect("FIAAS_ENV must be set")
        .as_ref()
    {
        "local" => FiaasEnv::Local,
        "dev" => FiaasEnv::Dev,
        "prod" => FiaasEnv::Prod,
        _ => panic!("FIAAS_ENV must be one of local, dev and prod"),
    };

    init(finn_app, env, level);
}
