use humantime::{self, Rfc3339Timestamp};
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

fn format_log_local(timestamp: &Rfc3339Timestamp, record: &Record) -> String {
    format!(
        "[{timestamp} {level} {logger}] {message}",
        timestamp = &timestamp,
        logger = record.target(),
        level = record.level(),
        message = record.args(),
    )
}

fn format_log_fiaas(timestamp: &Rfc3339Timestamp, record: &Record, finn_app: &str) -> String {
    let t = thread::current();
    serde_json::to_string(&json!({
      "@version":1,
      "@timestamp": &timestamp.to_string(),
      "logger": record.target(),
      "thread": format!("{}-{:?}", &t.name().unwrap_or("unnamed"), &t.id()),
      "level": record.level().to_string(),
      "message": record.args(),
      "finn_app": finn_app,
    }))
    .unwrap()
}

impl Log for FiaasLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = humantime::format_rfc3339_millis(SystemTime::now());
            match self.env {
                FiaasEnv::Local => {
                    let message = format_log_local(&timestamp, record);
                    match record.level() {
                        Level::Error => eprintln!("{}", &message),
                        _ => println!("{}", &message),
                    }
                }
                FiaasEnv::Dev | FiaasEnv::Prod => {
                    let message = format_log_fiaas(&timestamp, record, &self.finn_app);
                    match record.level() {
                        Level::Error => eprintln!("{}", &message),
                        _ => println!("{}", &message),
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

    let env = match std::env::var("FIAAS_ENVIRONMENT")
        .expect("FIAAS_ENVIRONMENT must be set")
        .as_ref()
    {
        "local" => FiaasEnv::Local,
        "dev" => FiaasEnv::Dev,
        "prod" => FiaasEnv::Prod,
        _ => panic!("FIAAS_ENVIRONMENT must be one of local, dev and prod"),
    };

    init(finn_app, env, level);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_env_works() {
        std::env::set_var("FIAAS_ENVIRONMENT", "local");
        std::env::set_var("RUST_LOG", "warn");
        init_env("test");
    }

    #[test]
    fn log_format_local_is_correct() {
        let timestamp = humantime::format_rfc3339_millis(SystemTime::now());
        let record = Record::builder()
            .args(format_args!("Error!"))
            .level(Level::Error)
            .target("test")
            .build();
        let produced_log = format_log_local(&timestamp, &record);
        let sample_log = format!("[{} ERROR test] Error!", timestamp);
        assert_eq!(sample_log, produced_log);
    }

    #[test]
    fn log_format_fiaas_is_correct() {
        let timestamp = humantime::format_rfc3339_millis(SystemTime::now());
        let record = Record::builder()
            .args(format_args!("Error!"))
            .level(Level::Error)
            .target("test")
            .build();
        let produced_log = format_log_fiaas(&timestamp, &record, "test");
        let sample_log = format!(
            "{{\
                \"@timestamp\":\"{}\",\
                \"@version\":1,\
                \"finn_app\":\"test\",\
                \"level\":\"ERROR\",\
                \"logger\":\"test\",\
                \"message\":\"Error!\",\
                \"thread\":\"tests::log_format_fiaas_is_correct-{:?}\"\
            }}",
            &timestamp,
            &thread::current().id()
        );
        assert_eq!(sample_log, produced_log);
    }
}
