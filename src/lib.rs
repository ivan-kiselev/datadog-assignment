// Load ./parser_combinators.rs
mod parser_combinators;
extern crate logwatcher;

use chrono::{Duration, Utc};
use logwatcher::{LogWatcher, LogWatcherAction};
pub use parser_combinators::*;
use std::io::BufRead;
use std::sync::mpsc::Sender;

// Return range of unix timestamps indicatding now() substracted by refresh interval
// We +1 in the end because Rust ranges are not inclusive on the right side..
// ..so as we calculate timestamp every iteration - it's often going to be..
// ..the same as timestamp of the log entry, e.g. log won't be included in the range
fn acceptible_timestamps(refresh_interval: Duration) -> std::ops::Range<i64> {
    (Utc::now().timestamp() - refresh_interval.num_seconds())..Utc::now().timestamp() + 1
}

pub fn read_logs(tx_stats: Sender<LogEntry>, refresh_interval: Duration, filename: &str) {
    // Panicing in both cases if can't open the file
    let file = std::fs::File::open(filename).unwrap();
    let mut log_watcher = LogWatcher::register(filename.to_string()).unwrap();

    // Initial file read on the start
    let lines = std::io::BufReader::new(&file).lines();
    for line in lines {
        if let Ok(unparsed_log) = line {
            if let Ok(log) = parsers::parse_log_entry(&unparsed_log[..]) {
                // Generate set of Unix timestamps according to refresh window and check
                // if the log entry is within this timestamp
                if acceptible_timestamps(refresh_interval).contains(&log.timestamp.timestamp()) {
                    tx_stats.send(log).unwrap();
                }
            }
        }
    }

    // Continious file read after the file was read initially, in case it being written to continiously
    log_watcher.watch(&mut move |line: String| {
        if let Ok(log) = parsers::parse_log_entry(&line[..]) {
            if acceptible_timestamps(refresh_interval).contains(&log.timestamp.timestamp()) {
                tx_stats.send(log).unwrap();
            }
        }
        LogWatcherAction::None
    });
}
