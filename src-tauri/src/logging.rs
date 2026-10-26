//! The command line options and the logs of swarmy. The logs go to the standard output and to a
//! file per day in the logs directory of the app, the same one Tauri uses:
//! - Linux: `$XDG_DATA_HOME/com.swarmy/logs`, i.e. `~/.local/share/com.swarmy/logs`
//! - Windows: `%LOCALAPPDATA%\com.swarmy\logs`
//! - macOS: `~/Library/Logs/com.swarmy`

use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::fmt::format::{DefaultFields, FmtSpan, Writer};
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

/// The identifier of tauri.conf.json, the directories of the app are named after it.
const APP_IDENTIFIER: &str = "com.swarmy";

/// The environment variable with the log filter, it takes precedence over `-v`.
pub const LOG_ENV: &str = "SWARMY_LOG";

/// The log files kept, one per day.
const MAX_LOG_FILES: usize = 7;

/// The fields of the log files, a type of its own so that they are not formatted once with the
/// colors of the standard output and reused.
#[derive(Default)]
struct FileFields(DefaultFields);

impl<'writer> FormatFields<'writer> for FileFields {
    fn format_fields<R: RecordFields>(
        &self,
        writer: Writer<'writer>,
        fields: R,
    ) -> std::fmt::Result {
        self.0.format_fields(writer, fields)
    }
}

/// StarCraft II replay analysis.
#[derive(Parser, Debug, Default)]
#[command(version, about, after_help = format!(
    "The logs are written to {}.\n\
     {LOG_ENV} takes a filter instead of -v, i.e. {LOG_ENV}=info,swarmy_tauri_lib=trace",
    log_dir().map(|dir| dir.display().to_string()).unwrap_or_default()
))]
pub struct Args {
    /// More logs: -v info, -vv debug and the time each step takes, -vvv trace.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

/// The directory of the log files.
pub fn log_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        dirs::home_dir().map(|home| home.join("Library/Logs").join(APP_IDENTIFIER))
    } else {
        dirs::data_local_dir().map(|dir| dir.join(APP_IDENTIFIER).join("logs"))
    }
}

/// The filter for the verbosity: swarmy itself logs info by default, the libraries warnings.
fn verbosity_filter(verbose: u8) -> &'static str {
    match verbose {
        0 => "warn,swarmy_tauri_lib=info",
        1 => "info",
        2 => "debug",
        _ => "trace",
    }
}

/// Logs to the standard output and to the log files. The time spans take, i.e. of the setup, the
/// plugins and the commands of the frontend, are logged from -vv.
pub fn init(args: &Args) {
    let filter = std::env::var(LOG_ENV)
        .ok()
        .filter(|filter| !filter.trim().is_empty())
        .unwrap_or_else(|| verbosity_filter(args.verbose).to_string());
    let span_events = if args.verbose >= 2 || std::env::var(LOG_ENV).is_ok() {
        FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };
    let file_layer = log_dir().and_then(|dir| {
        tracing_appender::rolling::Builder::new()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("swarmy")
            .filename_suffix("log")
            .max_log_files(MAX_LOG_FILES)
            .build(&dir)
            .inspect_err(|err| eprintln!("No log files in {}: {err}", dir.display()))
            .ok()
            .map(|appender| {
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .fmt_fields(FileFields::default())
                    .with_span_events(span_events.clone())
                    .with_writer(appender)
            })
    });
    let result = tracing_subscriber::registry()
        .with(EnvFilter::try_new(&filter).unwrap_or_else(|err| {
            eprintln!("Invalid {LOG_ENV} filter {filter:?}: {err}");
            EnvFilter::new(verbosity_filter(args.verbose))
        }))
        .with(tracing_subscriber::fmt::layer().with_span_events(span_events))
        .with(file_layer)
        .try_init();
    if let Err(err) = result {
        eprintln!("The logs are already configured: {err}");
    }
    tracing::info!(
        "swarmy {} logging with filter {filter:?} to {:?}",
        env!("CARGO_PKG_VERSION"),
        log_dir()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_the_verbosity() {
        assert_eq!(Args::parse_from(["swarmy"]).verbose, 0);
        assert_eq!(Args::parse_from(["swarmy", "-vv"]).verbose, 2);
        assert_eq!(Args::parse_from(["swarmy", "-v", "--verbose"]).verbose, 2);
        for verbose in 0..4 {
            assert!(EnvFilter::try_new(verbosity_filter(verbose)).is_ok());
        }
        assert!(log_dir().is_some_and(|dir| dir.ends_with("logs") || cfg!(target_os = "macos")));
    }
}
