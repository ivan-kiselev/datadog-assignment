extern crate clf_parser;
use chrono::{Duration, Utc};
use clap::Clap;
use clf_parser::*;
use rbl_circular_buffer::*;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

/// Utility to parse a CLF log file
/// reflects some statistics about rate and different endpoints
#[derive(Clap)]
#[clap(version = "0.1", author = "Ivan Kiselev. <kiselev_ivan@pm.me>")]
struct Opts {
    /// Sets an interval of refreshing the statistics in seconds, max 256
    #[clap(short, long, default_value = "10")]
    refresh_interval: u64,
    /// Alerts after this much seconds traffic being more than treshold on average, treshold is set by alert_treshold.
    #[clap(long, default_value = "20")]
    alert_interval: u64,
    /// Treshold for the alert
    #[clap(long, default_value = "1")]
    alert_treshold: u64,
    /// Path to the file to watch
    #[clap(short, long, default_value = "/tmp/access.log")]
    filename: String,
}

fn main() {
    let opts: Opts = Opts::parse();
    let refresh_interval = opts.refresh_interval;
    let filename = opts.filename;
    let (tx_stats, rx_stats): (Sender<LogEntry>, Receiver<LogEntry>) = mpsc::channel();
    thread::spawn(move || {
        read_logs(
            tx_stats,
            Duration::seconds(refresh_interval as i64),
            &filename[..],
        );
    });

    // A circular buffer for collect alerting values
    // pushing to CircularBuffer<T> of len that equels its max capacity results in replacing first element
    // making it simple FIFO perfectly suitable for keeping values exclusively relevant for alerting
    let max_alerting_buffer_len = if opts.alert_interval % refresh_interval != 0 {
        opts.alert_interval / refresh_interval + 1
    } else {
        opts.alert_interval / refresh_interval
    } as usize;
    let mut alerting_buffer: CircularBuffer<i64> = CircularBuffer::new(max_alerting_buffer_len);

    // Hashmap with {section => hit_rate}, where section is '/users'
    let mut stats: HashMap<String, i64> = HashMap::new();

    // Start counter for watching average values over time
    let mut start = Utc::now().timestamp() as u64;

    loop {
        let message = rx_stats.recv_timeout(std::time::Duration::from_secs(opts.refresh_interval));
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
                    opts.refresh_interval
                )
            }
        }
        let end = Utc::now().timestamp() as u64;
        if (end - start) >= opts.refresh_interval {
            start = Utc::now().timestamp() as u64;
            println!();
            let mut total = 0;
            for (key, value) in &stats {
                println!("Endpoint {} was hit {} times!", key, value);
                total += value;
            }
            // Save amount of requests for the last refresh_interval to compare with alerting_treshold later
            alerting_buffer.push(total);
            if alerting_buffer.len() == max_alerting_buffer_len {
                // Calculate average hits over alert_interval
                let avg_val_in_alert_interval = alerting_buffer
                    .collect::<Vec<i64>>()
                    .into_iter()
                    .sum::<i64>()
                    / (max_alerting_buffer_len * opts.refresh_interval as usize) as i64;
                if avg_val_in_alert_interval >= opts.alert_treshold as i64 {
                    println!(
                        "The rate has exceeded treshold, the value: {}!",
                        avg_val_in_alert_interval
                    );
                };
            }
            stats = HashMap::default();
        }
    }
}
