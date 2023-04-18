use anyhow::Context;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;

pub fn setup_tracing() -> anyhow::Result<()> {
    let rotation = Rotation::DAILY;
    let file_appender = RollingFileAppender::new(rotation, "logs", "trace.log");

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .with_writer(non_blocking)
        .finish();

    // Set the subscriber as the global tracing subscriber.
    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set global tracing subscriber")
}
