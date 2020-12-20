use chrono::{Duration, Utc};
use std::io::BufRead;
use std::sync::mpsc::Sender;
extern crate logwatcher;
use logwatcher::{LogWatcher, LogWatcherAction};
use std::collections::HashMap;

// Load ./parser_combinators.rs
mod parser_combinators;
pub use crate::parser_combinators::parsers;

// Return range of unix timestamps indicatding now() substracted by refresh interval
// We +1 in the end because Rust ranges are not inclusive..
// ..so as we calculate timestamp every iteration - it's often going to be..
// ..the same as timestamp of the log entry, e.g. log won't be included in the range
fn acceptible_timestamps(refresh_interval: Duration) -> std::ops::Range<i64> {
    (Utc::now().timestamp() - refresh_interval.num_seconds())..Utc::now().timestamp() + 1
}

// Sum of all the keys for all the timestamps in the stats HashMap
fn count_logs_in_interval(stats: &HashMap<i64, i64>) -> i64 {
    let mut count = 0;
    for value in stats.values() {
        count += value;
    }
    count
}

pub fn read_logs(channel: Sender<i64>, refresh_interval: Duration, filename: &str) {
    let file = std::fs::File::open(filename).unwrap();
    let mut log_watcher = LogWatcher::register(filename.to_string()).unwrap();
    let mut stats: HashMap<i64, i64> = HashMap::new();

    // Initial file read on the start
    let lines = std::io::BufReader::new(&file).lines();
    for line in lines {
        match line {
            Ok(unparsed_log) => {
                match parsers::parse_log_entry(&unparsed_log[..]) {
                    Ok(log) => {
                        // Generate set of Unix timestamps according to refresh window
                        if acceptible_timestamps(refresh_interval)
                            .contains(&log.timestamp.timestamp())
                        {
                            // Check if log entry's timestamp is within previously generated timestamps HashMap
                            let count = stats.entry(log.timestamp.timestamp()).or_insert(0);
                            *count += 1;
                        }
                    }
                    Err(err) => println!("Error: {}", err),
                }
            }
            Err(err) => println!("Error: {}", err),
        }
    }

    // Clean HashMatimestampp from timestamps that are too old already after initial read
    stats.retain(|key, _| acceptible_timestamps(refresh_interval).contains(key));
    channel.send(count_logs_in_interval(&stats)).unwrap();

    // Continious file read after the file was read initially, in case it being written to continiously
    log_watcher.watch(&mut move |line: String| {
        match parsers::parse_log_entry(&line[..]) {
            Ok(log) => {
                let timestamps = acceptible_timestamps(refresh_interval);
                if timestamps.contains(&log.timestamp.timestamp()) {
                    let count = stats.entry(log.timestamp.timestamp()).or_insert(0);
                    *count += 1;
                }
                stats.retain(|key, _| timestamps.contains(key));
                channel.send(count_logs_in_interval(&stats)).unwrap();
            }
            Err(err) => println!("{}", err),
        }
        LogWatcherAction::None
    });
}
