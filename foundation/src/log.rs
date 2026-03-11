//! Minimal logger integration built on top of the [`log`] crate.
//!
//! The logger is intentionally simple:
//!
//! - filtering is controlled by a single [`log::LevelFilter`]
//! - records are written as plain text to standard error
//! - initialization installs the logger as the process-global `log` backend

use crate::rust_alloc::boxed::Box;
use crate::rust_alloc::format;
use std::io::{self, Write};

pub use ::log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

/// A minimal logger that writes enabled records to standard error.
pub struct Logger {
    level: LevelFilter,
}

/// Creates a new [`Logger`] with the provided maximum level.
pub const fn logger(level: LevelFilter) -> Logger {
    Logger::new(level)
}

impl Logger {
    /// Creates a new logger that enables records up to `level`.
    pub const fn new(level: LevelFilter) -> Self {
        Self { level }
    }

    /// Returns the maximum level accepted by this logger.
    pub const fn level(&self) -> LevelFilter {
        self.level
    }

    /// Installs this logger as the global [`log`] implementation.
    ///
    /// This can only succeed once per process.
    pub fn init(self) -> Result<(), SetLoggerError> {
        let level = self.level;
        let logger = Box::leak(Box::new(self));
        ::log::set_logger(logger).map(|()| ::log::set_max_level(level))
    }

    fn format(record: &Record<'_>) -> crate::rust_alloc::string::String {
        format!("[{} {}] {}", record.level(), record.target(), record.args())
    }
}

impl ::log::Log for Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let mut stderr = io::stderr().lock();
        let _ = writeln!(stderr, "{}", Self::format(record));
    }

    fn flush(&self) {
        let _ = io::stderr().lock().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::Logger;
    use ::log::{Level, LevelFilter, Log, Metadata, Record};

    #[test]
    fn enabled_respects_the_configured_level() {
        let logger = Logger::new(LevelFilter::Info);
        let info = Metadata::builder()
            .level(Level::Info)
            .target("test")
            .build();
        let debug = Metadata::builder()
            .level(Level::Debug)
            .target("test")
            .build();

        assert!(logger.enabled(&info));
        assert!(!logger.enabled(&debug));
    }

    #[test]
    fn format_includes_level_target_and_message() {
        let record = Record::builder()
            .level(Level::Warn)
            .target("foundation::tests")
            .args(format_args!("disk almost full"))
            .build();

        assert_eq!(
            Logger::format(&record),
            "[WARN foundation::tests] disk almost full"
        );
    }
}
