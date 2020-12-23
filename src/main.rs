extern crate clf_parser;
use chrono::Duration;
use clap::Clap;
use clf_parser::*;
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
    collect_stats(
        rx_stats,
        opts.refresh_interval,
        opts.alert_interval,
        opts.alert_treshold,
    );
}
