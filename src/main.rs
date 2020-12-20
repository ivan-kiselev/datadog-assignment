extern crate clf_parser;
use chrono::{Duration, Utc};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

fn main() {
    let refresh_interval: i64 = 10;
    let (tx_stats, rx_stats): (Sender<i64>, Receiver<i64>) = mpsc::channel();
    thread::spawn(move || {
        clf_parser::read_logs(
            tx_stats,
            Duration::seconds(refresh_interval),
            "/tmp/access.log",
        );
    });

    loop {
        let start = Utc::now().timestamp() as u64;
        let message = rx_stats.recv();
        match message {
            Ok(messages_count) => println!(
                "Rate: {}/s",
                messages_count as f64 / refresh_interval as f64
            ),
            Err(err) => println!("Error: {}", err),
        }
        let end = Utc::now().timestamp() as u64;
        thread::sleep(std::time::Duration::from_secs(
            refresh_interval as u64 - (end - start),
        ));
    }
}
