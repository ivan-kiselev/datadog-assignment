// use chrono::format::ParseError;
use chrono::{DateTime, FixedOffset};
use std::fmt;
use std::fmt::Display;
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug)]
pub enum IPAddress {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

pub struct Request {
    pub method: u16,
    pub path: String,
}

// Example of the time log:
// 127.0.0.1 user-identifier frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326
// #[derive(Clone, Default, Debug)]
pub struct LogEntry {
    pub ip_address: IPAddress,
    pub user_agent: String,
    pub user_id: String,
    pub timestamp: DateTime<FixedOffset>,
    pub request: Request,
    pub response_code: u16,
    pub size: u128,
}

impl Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method, self.path)
    }
}

impl Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\n
            ip_address: {:?}\n
            user_agent: {} \n
            user_id: {}\n
            timestamp: {} \n
            request: {}\n
            response_code: {}\n
            size: {}\n",
            self.ip_address,
            self.user_agent,
            self.user_id,
            self.timestamp,
            self.request,
            self.response_code,
            self.size
        )
    }
}
pub(self) mod parsers {

    #[cfg(test)]
    mod tests {
        use super::*;
    }
}
