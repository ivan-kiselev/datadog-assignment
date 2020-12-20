extern crate clf_parser;
use chrono::{Duration, Utc};
use clap::Clap;
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
    interval: u8,
    /// Path to the file to watch
    #[clap(short, long, default_value = "/tmp/access.log")]
    filename: String,
}

fn main() {
    let opts: Opts = Opts::parse();
    let refresh_interval = opts.interval;
    let filename = opts.filename;
    let (tx_stats, rx_stats): (Sender<i64>, Receiver<i64>) = mpsc::channel();
    thread::spawn(move || {
        clf_parser::read_logs(
            tx_stats,
            Duration::seconds(refresh_interval as i64),
            &filename[..],
        );
    });

    loop {
        let start = Utc::now().timestamp() as u64;
        let message = rx_stats.recv();
        match message {
            Ok(messages_count) => {
                println!("Rate: {}/s", messages_count as f64 / opts.interval as f64)
            }
            Err(err) => println!("Error: {}", err),
        }
        let end = Utc::now().timestamp() as u64;
        thread::sleep(std::time::Duration::from_secs(
            // Prevent overflow in case we didn't have logs for time more than refresh interval
            if (opts.interval as u64) >= end - start {
                opts.interval as u64 - (end - start)
            } else {
                0
            },
        ));
    }
}

/*  let start = Utc::now().timestamp() as u64;
let message = rx_stats.recv();
...
let end = Utc::now().timestamp() as u64;
thread::sleep(std::time::Duration::from_secs(
    opts.interval as u64 - (end - start),
)); */
