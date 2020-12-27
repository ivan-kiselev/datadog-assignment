// Load ./parser_combinators.rs
mod parser_combinators;
mod ui;
extern crate logwatcher;

use chrono::{Duration, Utc};
use logwatcher::{LogWatcher, LogWatcherAction};
pub use parser_combinators::*;
use rbl_circular_buffer::*;
use std::process::exit;
use std::{
    collections::HashMap,
    io,
    io::BufRead,
    net::IpAddr,
    sync::mpsc::{Receiver, SendError, Sender},
};
use termion::{event::Key, input::TermRead};

pub use ui::{draw, init_ui, RenderMessage, UIUpdate};

// Return range of unix timestamps indicatding now() substracted by refresh interval
// We +1 in the end because Rust ranges are not inclusive on the right side..
// ..so as we calculate timestamp every iteration - it's often going to be..
// ..the same as timestamp of the log entry, e.g. log won't be included in the range
fn acceptible_timestamps(refresh_interval: Duration) -> std::ops::Range<i64> {
    (Utc::now().timestamp() - refresh_interval.num_seconds())..Utc::now().timestamp() + 1
}

// Read logs and send log entries through Sender (tx_stats)
pub fn read_logs(
    follow: bool,
    tx_logs: Sender<LogEntry>,
    refresh_interval: Duration,
    filename: &str,
) {
    // Panicing in both cases if can't open the file
    if !follow {
        // Initial file read on the start
        match std::fs::File::open(filename) {
            Ok(file) => {
                let lines = std::io::BufReader::new(&file).lines();
                for line in lines {
                    if let Ok(unparsed_log) = line {
                        if let Ok(log) = parsers::parse_log_entry(&unparsed_log[..]) {
                            // Generate set of Unix timestamps according to refresh window and check
                            // if the log entry is within this timestamp
                            if acceptible_timestamps(refresh_interval)
                                .contains(&log.timestamp.timestamp())
                            {
                                tx_logs.send(log).unwrap();
                            }
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("Could not read the file: {}", err);
                exit(1);
            }
        }
    }

    // Continious file read after the file was read initially, in case it being written to continiously
    match LogWatcher::register(filename.to_string()).as_mut() {
        Ok(log_watcher) => {
            log_watcher.watch(&mut move |line: String| {
                if let Ok(log) = parsers::parse_log_entry(&line[..]) {
                    tx_logs.send(log).unwrap();
                }
                LogWatcherAction::None
            });
        }
        Err(err) => {
            println!("Could not read the file: {}", err);
            exit(1);
        }
    }
}

// Receive logs and aggregate them to data
pub fn collect_stats(
    rx_logs: Receiver<LogEntry>,
    refresh_interval: u64,
    alerting_interval: u64,
    alert_threshold: u64,
    tx_stats: Sender<RenderMessage>,
    http_codes: Vec<u16>,
) {
    // Hashmap with {section => hit_rate}, where section is '/users'
    let mut stats_endpoints: HashMap<String, u64> = HashMap::new();
    let mut stats_addresses: HashMap<IpAddr, u64> = HashMap::new();
    let mut stats_http_codes: HashMap<u16, HashMap<String, u64>> = HashMap::new();
    let mut log_samples: CircularBuffer<String> = CircularBuffer::new(10);
    // A circular buffer for collect alerting values
    // pushing to CircularBuffer<T> of len that equels its max capacity results in replacing first element
    // making it simple FIFO perfectly suitable for keeping values exclusively relevant for alerting
    let max_alerting_buffer_len = if alerting_interval % refresh_interval != 0 {
        alerting_interval / refresh_interval + 1
    } else {
        alerting_interval / refresh_interval
    } as usize;
    let mut alerting_buffer: CircularBuffer<u64> = CircularBuffer::new(max_alerting_buffer_len);
    let recieve_timeout = std::time::Duration::from_secs(refresh_interval);
    // Start counter for watching average values over time
    let mut start = Utc::now().timestamp() as u64;
    loop {
        let message = rx_logs.recv_timeout(recieve_timeout);
        if let Ok(log_entry) = message {
            // Increment endpoint hits
            let count_endpoints = stats_endpoints
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
            *count_endpoints += 1;

            // Increment IP address requests count
            let count_addresses = stats_addresses.entry(log_entry.ip_address).or_insert(0);
            *count_addresses += 1;

            // Increment HTTP codes requests count if the code in the list to watch
            if http_codes.iter().any(|&i| i == log_entry.response_code) {
                let count_http_codes = stats_http_codes
                    .entry(log_entry.response_code)
                    .or_insert_with(HashMap::new);
                let count_http_codes_path = count_http_codes
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
                *count_http_codes_path += 1;
            };

            // Collect 10 Log samples into FIFO buffer of limited capacity
            log_samples.push(log_entry.to_string());
        }
        let end = Utc::now().timestamp() as u64;
        if (end - start) >= refresh_interval {
            start = Utc::now().timestamp() as u64;
            let mut total = 0;
            for value in stats_endpoints.values() {
                total += value;
            }
            // Save amount of requests for the last refresh_interval to compare with alerting_treshold later
            alerting_buffer.push(total);
            let mut render_message = RenderMessage::UI(UIUpdate {
                stats_endpoints,
                avg_rate: total as u64 / refresh_interval,
                treshold_reached: false,
                stats_addresses: stats_addresses.clone(),
                log_samples: log_samples.to_owned().collect::<Vec<String>>(),
                avg_within_alert_interval: 0,
                stats_http_codes,
            });
            if alerting_buffer.len() == max_alerting_buffer_len {
                // Calculate average hits over alert_interval
                let avg_val_in_alert_interval = alerting_buffer
                    .collect::<Vec<u64>>()
                    .into_iter()
                    .sum::<u64>()
                    / (max_alerting_buffer_len * refresh_interval as usize) as u64;
                if avg_val_in_alert_interval >= alert_threshold as u64 {
                    if let RenderMessage::UI(ref mut message) = render_message {
                        message.treshold_reached = true;
                        message.avg_within_alert_interval = avg_val_in_alert_interval;
                    }
                };
            }
            tx_stats.send(render_message).unwrap();
            // Refresh stats between refresh intervals
            stats_endpoints = HashMap::default();
            stats_addresses = HashMap::default();
            stats_http_codes = HashMap::default();
        }
    }
}

pub fn keyboard_listener(
    tx_keyboard: Sender<RenderMessage>,
) -> Result<(), SendError<RenderMessage>> {
    let stdin = io::stdin();
    for evt in stdin.keys() {
        if let Ok(key) = evt {
            if key == Key::Char('q') {
                tx_keyboard.send(RenderMessage::Exit)?
            }
        }
    }
    Ok(())
}
