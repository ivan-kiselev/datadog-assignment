extern crate clf_parser;
use chrono::{Duration, Utc};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

fn main() {
    let (tx_stats, rx_stats): (Sender<i64>, Receiver<i64>) = mpsc::channel();
    let _ = thread::spawn(move || {
        clf_parser::read_logs(tx_stats, Duration::seconds(10), "/tmp/access.log");
    });

    loop {
        let start = Utc::now().timestamp() as u64;
        let message = rx_stats.recv();
        match message {
            Ok(messages_count) => println!("Rate: {}/s", messages_count as f64 / 10.0),
            Err(err) => println!("Error: {}", err),
        }
        let end = Utc::now().timestamp() as u64;
        thread::sleep(std::time::Duration::from_secs(10 - (end - start)));
    }
}
