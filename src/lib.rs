// Load ./parser_combinators.rs
mod parser_combinators;
mod ui;
extern crate logwatcher;

use chrono::{Duration, Utc};
use logwatcher::{LogWatcher, LogWatcherAction};
pub use parser_combinators::*;
use rbl_circular_buffer::*;
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::{Receiver, Sender};
pub use ui::{draw, init_ui, Alert, RenderMessage, UIUpdate};

// Return range of unix timestamps indicatding now() substracted by refresh interval
// We +1 in the end because Rust ranges are not inclusive on the right side..
// ..so as we calculate timestamp every iteration - it's often going to be..
// ..the same as timestamp of the log entry, e.g. log won't be included in the range
fn acceptible_timestamps(refresh_interval: Duration) -> std::ops::Range<i64> {
    (Utc::now().timestamp() - refresh_interval.num_seconds())..Utc::now().timestamp() + 1
}

// Read logs and send log entries through Sender (tx_stats)
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

// Receive logs and aggregate them to data
pub fn collect_stats(
    rx_stats: Receiver<LogEntry>,
    refresh_interval: u64,
    alerting_interval: u64,
    alert_threshold: u64,
    tx_stats: Sender<RenderMessage>,
) {
    // Hashmap with {section => hit_rate}, where section is '/users'
    let mut stats: HashMap<String, i64> = HashMap::new();

    // A circular buffer for collect alerting values
    // pushing to CircularBuffer<T> of len that equels its max capacity results in replacing first element
    // making it simple FIFO perfectly suitable for keeping values exclusively relevant for alerting
    let max_alerting_buffer_len = if alerting_interval % refresh_interval != 0 {
        alerting_interval / refresh_interval + 1
    } else {
        alerting_interval / refresh_interval
    } as usize;
    let mut alerting_buffer: CircularBuffer<i64> = CircularBuffer::new(max_alerting_buffer_len);
    let recieve_timeout = std::time::Duration::from_secs(refresh_interval);
    // Start counter for watching average values over time
    let mut start = Utc::now().timestamp() as u64;
    loop {
        let message = rx_stats.recv_timeout(recieve_timeout);
        match message {
            Ok(log_entry) => {
                let count = stats
                    .entry(
                        log_entry
                            .request
                            .path
                            .clone()
                            .split('/')
                            .collect::<Vec<&str>>()[1]
                            .to_owned(),
                    )
                    .or_insert(0);
                *count += 1;
            }
            Err(RecvTimeoutError::Disconnected) => {
                println!("File-reading thread has failed, check file path and permissions")
            }
            Err(RecvTimeoutError::Timeout) => {
                println!(
                    "Logs stopped coming, timeout of {} seconds occured!",
                    refresh_interval
                )
            }
        }
        let end = Utc::now().timestamp() as u64;
        if (end - start) >= refresh_interval {
            start = Utc::now().timestamp() as u64;
            let mut total = 0;
            for (_, value) in &stats {
                total += value;
            }
            // Save amount of requests for the last refresh_interval to compare with alerting_treshold later
            alerting_buffer.push(total);
            let mut render_message = RenderMessage::UI(UIUpdate {
                stats: stats,
                alert: Alert {
                    active: false,
                    value: 0,
                },
            });
            if alerting_buffer.len() == max_alerting_buffer_len {
                // Calculate average hits over alert_interval
                let avg_val_in_alert_interval = alerting_buffer
                    .collect::<Vec<i64>>()
                    .into_iter()
                    .sum::<i64>()
                    / (max_alerting_buffer_len * refresh_interval as usize) as i64;
                if let RenderMessage::UI(ref mut message) = render_message {
                    message.alert.value = avg_val_in_alert_interval;
                }
                if avg_val_in_alert_interval >= alert_threshold as i64 {
                    if let RenderMessage::UI(ref mut message) = render_message {
                        message.alert.active = true;
                    }
                };
            }
            tx_stats.send(render_message).unwrap();
            stats = HashMap::default();
        }
    }
}
