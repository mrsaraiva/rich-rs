#![doc = include_str!("../README.md")]

#[cfg(any(feature = "log", feature = "tracing"))]
use rich_rs::{SimpleColor, Style, Text};

#[cfg(any(feature = "log", feature = "tracing"))]
fn style_for_level_name(level: &str) -> Style {
    match level {
        "ERROR" => Style::new()
            .with_color(SimpleColor::Standard(1))
            .with_bold(true),
        "WARN" => Style::new()
            .with_color(SimpleColor::Standard(3))
            .with_bold(true),
        "INFO" => Style::new().with_color(SimpleColor::Standard(6)),
        "DEBUG" => Style::new().with_color(SimpleColor::Standard(2)),
        "TRACE" => Style::new().with_dim(true),
        _ => Style::new(),
    }
}

#[cfg(any(feature = "log", feature = "tracing"))]
fn build_event_text(
    level_name: &str,
    message: &str,
    target: Option<&str>,
    fields: &[String],
) -> Text {
    let mut text = Text::styled(format!("[{level_name}]"), style_for_level_name(level_name));
    text.append(" ", None);
    text.append(message.to_string(), None);

    if let Some(target) = target {
        text.append(" ", None);
        text.append(
            format!("target={target}"),
            Some(Style::new().with_dim(true)),
        );
    }

    if !fields.is_empty() {
        text.append(" ", None);
        text.append(fields.join(" "), Some(Style::new().with_dim(true)));
    }

    text
}

#[cfg(feature = "tracing")]
mod tracing_adapter {
    use super::build_event_text;
    use std::fmt;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use rich_rs::Console;
    use tracing::field::{Field, Visit};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer};

    pub struct RichTracingLayer<W: Write + Send + 'static> {
        console: Arc<Mutex<Console<W>>>,
        include_target: bool,
        include_location: bool,
    }

    impl<W: Write + Send + 'static> Clone for RichTracingLayer<W> {
        fn clone(&self) -> Self {
            Self {
                console: self.console.clone(),
                include_target: self.include_target,
                include_location: self.include_location,
            }
        }
    }

    impl<W: Write + Send + 'static> RichTracingLayer<W> {
        pub fn new(console: Arc<Mutex<Console<W>>>) -> Self {
            Self {
                console,
                include_target: true,
                include_location: true,
            }
        }

        pub fn with_target(mut self, include_target: bool) -> Self {
            self.include_target = include_target;
            self
        }

        pub fn with_location(mut self, include_location: bool) -> Self {
            self.include_location = include_location;
            self
        }
    }

    #[derive(Default)]
    struct EventVisitor {
        message: Option<String>,
        fields: Vec<String>,
    }

    impl EventVisitor {
        fn normalize_debug(value: String) -> String {
            if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                value[1..value.len() - 1].to_string()
            } else {
                value
            }
        }

        fn push_field(&mut self, name: &str, value: String) {
            if name == "message" {
                self.message = Some(Self::normalize_debug(value));
            } else {
                self.fields.push(format!("{name}={value}"));
            }
        }
    }

    impl Visit for EventVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
            self.push_field(field.name(), format!("{value:?}"));
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.push_field(field.name(), value.to_string());
        }

        fn record_bool(&mut self, field: &Field, value: bool) {
            self.push_field(field.name(), value.to_string());
        }

        fn record_i64(&mut self, field: &Field, value: i64) {
            self.push_field(field.name(), value.to_string());
        }

        fn record_u64(&mut self, field: &Field, value: u64) {
            self.push_field(field.name(), value.to_string());
        }

        fn record_f64(&mut self, field: &Field, value: f64) {
            self.push_field(field.name(), value.to_string());
        }
    }

    impl<S, W> Layer<S> for RichTracingLayer<W>
    where
        S: Subscriber,
        W: Write + Send + 'static,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let metadata = event.metadata();
            let mut visitor = EventVisitor::default();
            event.record(&mut visitor);

            let message = visitor.message.unwrap_or_else(|| "event".to_string());
            let text = build_event_text(
                metadata.level().as_str(),
                &message,
                self.include_target.then_some(metadata.target()),
                &visitor.fields,
            );

            let file = self.include_location.then_some(metadata.file()).flatten();
            let line = self.include_location.then_some(metadata.line()).flatten();

            if let Ok(mut console) = self.console.lock() {
                let _ = console.log(&text, file, line);
            }
        }
    }

    pub use RichTracingLayer as PublicRichTracingLayer;
}

#[cfg(feature = "tracing")]
pub use tracing_adapter::PublicRichTracingLayer as RichTracingLayer;

#[cfg(feature = "log")]
mod log_adapter {
    use super::build_event_text;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};
    use rich_rs::Console;

    pub struct RichLogger<W: Write + Send + 'static> {
        console: Arc<Mutex<Console<W>>>,
        level_filter: LevelFilter,
        include_target: bool,
        include_location: bool,
    }

    impl<W: Write + Send + 'static> RichLogger<W> {
        pub fn new(console: Arc<Mutex<Console<W>>>) -> Self {
            Self {
                console,
                level_filter: LevelFilter::Trace,
                include_target: true,
                include_location: true,
            }
        }

        pub fn with_level_filter(mut self, level_filter: LevelFilter) -> Self {
            self.level_filter = level_filter;
            self
        }

        pub fn with_target(mut self, include_target: bool) -> Self {
            self.include_target = include_target;
            self
        }

        pub fn with_location(mut self, include_location: bool) -> Self {
            self.include_location = include_location;
            self
        }
    }

    impl<W: Write + Send + 'static> Log for RichLogger<W> {
        fn enabled(&self, metadata: &Metadata<'_>) -> bool {
            metadata.level() <= self.level_filter
        }

        fn log(&self, record: &Record<'_>) {
            if !self.enabled(record.metadata()) {
                return;
            }

            let text = build_event_text(
                record.level().as_str(),
                &record.args().to_string(),
                self.include_target.then_some(record.target()),
                &[],
            );

            let file = self.include_location.then_some(record.file()).flatten();
            let line = self.include_location.then_some(record.line()).flatten();

            if let Ok(mut console) = self.console.lock() {
                let _ = console.log(&text, file, line);
            }
        }

        fn flush(&self) {}
    }

    pub fn init_global_logger(logger: RichLogger<std::io::Stdout>) -> Result<(), SetLoggerError> {
        let level_filter = logger.level_filter;
        let logger: &'static RichLogger<std::io::Stdout> = Box::leak(Box::new(logger));
        log::set_logger(logger)?;
        log::set_max_level(level_filter);
        Ok(())
    }

    pub use RichLogger as PublicRichLogger;
}

#[cfg(feature = "log")]
pub use log_adapter::PublicRichLogger as RichLogger;
#[cfg(feature = "log")]
pub use log_adapter::init_global_logger;

#[cfg(test)]
mod tests {
    #[cfg(feature = "tracing")]
    #[test]
    fn tracing_layer_writes_to_rich_console() {
        use std::sync::{Arc, Mutex};

        use crate::RichTracingLayer;
        use rich_rs::Console;
        use tracing_subscriber::prelude::*;

        let console = Arc::new(Mutex::new(Console::capture()));
        let layer = RichTracingLayer::new(console.clone())
            .with_target(true)
            .with_location(false);
        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "service::http", request_id = 42, "request handled");
        });

        let output = console
            .lock()
            .expect("console lock poisoned")
            .get_captured();
        assert!(output.contains("[INFO]"), "output was: {output}");
        assert!(output.contains("request handled"), "output was: {output}");
        assert!(output.contains("request_id=42"), "output was: {output}");
        assert!(
            output.contains("target=service::http"),
            "output was: {output}"
        );
    }

    #[cfg(feature = "log")]
    #[test]
    fn log_logger_writes_to_rich_console() {
        use std::sync::{Arc, Mutex};

        use crate::RichLogger;
        use log::{Level, LevelFilter, Log, Record};
        use rich_rs::Console;

        let console = Arc::new(Mutex::new(Console::capture()));
        let logger = RichLogger::new(console.clone())
            .with_level_filter(LevelFilter::Info)
            .with_target(true)
            .with_location(true);

        let args = format_args!("database connected");
        let record = Record::builder()
            .args(args)
            .level(Level::Info)
            .target("service::db")
            .file(Some("src/main.rs"))
            .line(Some(21))
            .build();

        Log::log(&logger, &record);

        let output = console
            .lock()
            .expect("console lock poisoned")
            .get_captured();
        assert!(output.contains("[INFO]"), "output was: {output}");
        assert!(
            output.contains("database connected"),
            "output was: {output}"
        );
        assert!(
            output.contains("target=service::db"),
            "output was: {output}"
        );
        assert!(output.contains("main.rs:21"), "output was: {output}");
    }
}
