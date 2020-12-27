use chrono::Duration;
use clap::Clap;
use clf_parser::*;
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
/// Utility to parse a CLF log file
/// reflects some statistics about rate and different endpoints
/// Developed in Rust 🦀 as an home-take assignment for DataDog interviewing process.
/// To exit application press 'q'
#[derive(Clap, Clone)]
#[clap(version = "0.1", author = "Ivan Kiselev. <kiselev_ivan@pm.me>")]
struct Opts {
    /// Sets an interval of refreshing the statistics in seconds, max 256
    #[clap(short, long, default_value = "10")]
    refresh_interval: u64,
    /// Alerts after this much seconds traffic being more than threshold on average, threshold is set by --alert-threshold.
    #[clap(long, default_value = "20")]
    alert_interval: u64,
    /// threshold for the alert
    #[clap(long, default_value = "10")]
    alert_threshold: u64,
    /// Path to the file to watch
    #[clap(short, long, default_value = "/tmp/access.log")]
    filename: String,
    /// Only follow the newly added content to file, do not read previously generated lines of code.
    /// Good for large files that are known for holding a lot of old logs
    #[clap(long)]
    follow_only: bool,
    /// HTTP response codes to reflect statistics on
    #[clap(
        short,
        long,
        min_values = 1,
        require_delimiter = true,
        default_value = "200,401,404,500,501,502,504"
    )]
    http_codes: Vec<u16>,
}

fn main() -> Result<(), io::Error> {
    let opts: Opts = Opts::parse();

    // Channel between log readre and stats producer
    let (tx_logs, rx_logs): (Sender<LogEntry>, Receiver<LogEntry>) = mpsc::channel();

    //Channel between stats producer and UI renderer
    let (tx_stats, rx_stats): (Sender<RenderMessage>, Receiver<RenderMessage>) = mpsc::channel();

    // Spawn thread to read and follow the file
    // Copy parameters as they are being consumed by threads
    let refresh_interval = opts.refresh_interval;
    let filename = opts.filename.clone();
    let follow = opts.follow_only;

    thread::spawn(move || {
        read_logs(
            follow,
            tx_logs,
            Duration::seconds(refresh_interval as i64),
            &filename[..],
        );
    });

    // Spawn thread to analyze logs and produce statistics
    // Copy parameters as they are being consumed by threads
    let refresh_interval = opts.refresh_interval;
    let alerting_interval = opts.alert_interval;
    let alerting_threshold = opts.alert_threshold;
    let stats_sender = tx_stats.clone();
    let http_codes = opts.http_codes;
    thread::spawn(move || {
        collect_stats(
            rx_logs,
            refresh_interval,
            alerting_interval,
            alerting_threshold,
            stats_sender,
            http_codes,
        )
    });
    tx_stats
        .send(RenderMessage::UI(UIUpdate::default()))
        .unwrap();
    thread::spawn(move || keyboard_listener(tx_stats));
    let mut screen = init_ui().unwrap();

    // Provoke instant rendering of the UI with default values instead of waiting for first cycle of refresh interval

    draw(
        &mut screen,
        rx_stats,
        opts.refresh_interval,
        opts.alert_interval,
        opts.alert_threshold,
        opts.filename,
    )?;
    Ok(())
}
