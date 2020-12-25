use chrono::Duration;
use clap::Clap;
use clf_parser::*;
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
/// Utility to parse a CLF log file
/// reflects some statistics about rate and different endpoints
#[derive(Clap, Clone)]
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

fn main() -> Result<(), io::Error> {
    let opts: Opts = Opts::parse();

    // Channel between log readre and stats producer
    let (tx_logs, rx_logs): (Sender<LogEntry>, Receiver<LogEntry>) = mpsc::channel();

    //Channel between stats producer and UI renderer
    let (tx_stats, rx_stats): (Sender<RenderMessage>, Receiver<RenderMessage>) = mpsc::channel();

    // Spawn thread to read and follow the file
    // Copy parameters as they are being consumed by threads
    //let filename = opts.filename.clone();
    let refresh_interval = opts.refresh_interval;
    let filename = opts.filename.clone();
    thread::spawn(move || {
        read_logs(
            tx_logs,
            Duration::seconds(refresh_interval as i64),
            &filename[..],
        );
    });

    // Spawn thread to analyze logs and produce statistics
    // Copy parameters as they are being consumed by threads
    let refresh_interval = opts.refresh_interval;
    let alerting_interval = opts.alert_interval;
    let alerting_treshold = opts.alert_treshold;
    thread::spawn(move || {
        collect_stats(
            rx_logs,
            refresh_interval,
            alerting_interval,
            alerting_treshold,
            tx_stats,
        )
    });
    let mut screen = init_ui().unwrap();
    draw(
        &mut screen,
        rx_stats,
        opts.refresh_interval.clone(),
        opts.alert_interval.clone(),
        opts.alert_treshold.clone(),
        opts.filename.clone(),
    )?;
    Ok(())
}
