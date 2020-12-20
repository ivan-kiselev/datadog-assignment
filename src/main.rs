extern crate clf_parser;
use chrono::{Duration, Utc};
use clap::Clap;
use clf_parser::*;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

/// Utility to parse a CLF log file
/// reflects some statistics about rate and different endpoints
#[derive(Clap)]
#[clap(version = "0.1", author = "Ivan Kiselev. <kiselev_ivan@pm.me>")]
struct Opts {
    /// Sets an interval of refreshing the statistics in seconds, max 256
    #[clap(short, long, default_value = "10")]
    interval: u64,
    /// Path to the file to watch
    #[clap(short, long, default_value = "/tmp/access.log")]
    filename: String,
}

fn main() {
    let opts: Opts = Opts::parse();
    let refresh_interval = opts.interval;
    let filename = opts.filename;
    let (tx_stats, rx_stats): (Sender<LogEntry>, Receiver<LogEntry>) = mpsc::channel();
    thread::spawn(move || {
        read_logs(
            tx_stats,
            Duration::seconds(refresh_interval as i64),
            &filename[..],
        );
    });
    let mut start = Utc::now().timestamp() as u64;
    let mut stats: HashMap<String, i64> = HashMap::new();
    loop {
        let message = rx_stats.recv_timeout(std::time::Duration::from_secs(opts.interval));
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
            Err(err) => println!("Logs stopped coming: {}!", err),
        }
        let end = Utc::now().timestamp() as u64;
        if (end - start) >= opts.interval {
            start = Utc::now().timestamp() as u64;
            println!("Stats for the last {} seconds:\n", opts.interval);
            for (key, value) in &stats {
                println!("Endpoint {} was hit {} times!", key, value);
            }

            stats = HashMap::default();
        }
    }
}
