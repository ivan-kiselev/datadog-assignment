extern crate clf_parser;
use chrono::Duration;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

fn main() {
    let (tx_stats, rx_stats): (Sender<i64>, Receiver<i64>) = mpsc::channel();
    let _ = thread::spawn(move || {
        clf_parser::read_logs(tx_stats, Duration::seconds(5), "/tmp/access.log");
    });

    loop {
        let message = rx_stats.recv();
        println!(
            "Logs Rate: {}",
            match message {
                Ok(c) => {
                    format!("{}", c as f64 / 5.0)
                }
                Err(err) => format!("{}", err),
            }
        )
    }
}
