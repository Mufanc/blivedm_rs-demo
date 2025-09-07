use crate::data::PROJECT_DIRS;
use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::io;
use std::io::Write;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt};

static G_LOGGER: Lazy<Logger> = Lazy::new(|| Logger::new());

struct Logger {
    _wgs: Vec<WorkerGuard>,
}

impl Logger {
    fn new() -> Self {
        let mut layers = Vec::new();
        let mut wgs = Vec::new();

        layers.push({
            let appender = rolling::daily(PROJECT_DIRS.data_dir().join("logs"), "logs.jsonl");
            let (non_blocking, wg) = tracing_appender::non_blocking(appender);

            wgs.push(wg);

            fmt::layer()
                .with_writer(non_blocking)
                .with_target(false)
                .with_file(false)
                .with_line_number(false)
                .with_ansi(false)
                .with_filter(LevelFilter::DEBUG)
                .boxed()
        });

        layers.push(
            fmt::layer()
                .with_writer(io::stdout)
                .with_target(false)
                .with_level(true)
                .with_file(true)
                .with_line_number(false)
                .with_ansi(true)
                .without_time()
                .with_filter(EnvFilter::from_default_env())
                .boxed(),
        );

        let subscriber = Registry::default().with(layers);

        subscriber.try_init().expect("failed to init tracer");

        Self { _wgs: wgs }
    }
}

pub struct MessageLogger {
    writer: NonBlocking,
    _wg: WorkerGuard,
}

impl MessageLogger {
    pub fn new(room_id: &str) -> Self {
        let appender = rolling::daily(PROJECT_DIRS.data_dir().join(room_id), "raw.jsonl");
        let (non_blocking, wg) = tracing_appender::non_blocking(appender);

        Self {
            writer: non_blocking,
            _wg: wg,
        }
    }

    pub fn write(&mut self, message: &Value) -> Result<()> {
        let message_str = message.to_string();
        self.writer.write_all(message_str.as_bytes())?;
        Ok(())
    }
}

pub fn init() {
    let _ = &*G_LOGGER;
}
