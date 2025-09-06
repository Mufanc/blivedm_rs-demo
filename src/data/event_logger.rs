use crate::data::PROJECT_DIRS;
use once_cell::sync::Lazy;
use std::time::SystemTime;
use std::{fmt, io};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry};

static G_TRACER: Lazy<Tracer> = Lazy::new(|| Tracer::new(&["raw", "danmaku"]));

struct UnixTimestamp;

impl FormatTime for UnixTimestamp {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time went backwards");

        write!(w, "{}", now.as_millis())
    }
}

fn build_layer(name: &str) -> (Box<dyn Layer<Registry> + Send + Sync>, WorkerGuard) {
    let appender = rolling::daily(PROJECT_DIRS.data_dir(), format!("{name}.jsonl"));
    let (non_blocking, wg) = tracing_appender::non_blocking(appender);

    let layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .json()
        .with_timer(UnixTimestamp)
        .with_target(false)
        .with_level(false)
        .with_span_list(false)
        .flatten_event(true)
        .with_filter(EnvFilter::new(format!("{name}=trace")))
        .boxed();

    (layer, wg)
}

struct Tracer {
    _wgs: Vec<WorkerGuard>
}

impl Tracer {
    fn new(layer_names: &[&str]) -> Self {
        let mut wgs = Vec::new();
        let mut layers = Vec::new();

        for name in layer_names {
            let (layer, wg) = build_layer(name);

            wgs.push(wg);
            layers.push(layer);
        }

        layers.push(
            tracing_subscriber::fmt::layer()
                .with_writer(io::stdout)
                .with_target(false)
                .with_level(true)
                .with_file(true)
                .with_line_number(false)
                .with_ansi(true)
                .without_time()
                .with_filter(EnvFilter::from_default_env())
                .boxed()
        );

        let subscriber = Registry::default().with(layers);

        subscriber.try_init().expect("failed to init tracer");

        Self { _wgs: wgs }
    }
}

pub fn init() {
    let _ = &*G_TRACER;
}
