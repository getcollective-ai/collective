use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};

/// Setup tracing to write to a file.
pub fn setup_tracing() -> WorkerGuard {
    let rotation = Rotation::DAILY;
    let file_appender = RollingFileAppender::new(rotation, "logs", "trace.log");

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt::Subscriber::builder()
        .with_writer(non_blocking)
        .init();

    guard
}
