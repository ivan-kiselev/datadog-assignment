// use chrono::format::ParseError;
use chrono::{DateTime, FixedOffset};
use std::fmt;
use std::fmt::Display;
use std::net::IpAddr;

pub struct Request {
    pub method: u16,
    pub path: String,
}

// Example of the time log:
// 127.0.0.1 user-identifier frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326
// #[derive(Clone, Default, Debug)]
pub struct LogEntry {
    pub ip_address: IpAddr,
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
    use nom::{error::ErrorKind, Err, IResult};
    use std::net::{AddrParseError, IpAddr};

    fn not_whitespace(i: &str) -> nom::IResult<&str, &str> {
        nom::bytes::complete::is_not(" \t")(i)
    }

    fn parse_ip_address(i: &str) -> nom::IResult<&str, IpAddr> {
        nom::combinator::map_parser(not_whitespace, ip_parser)(i)
    }

    fn ip_parser(i: &str) -> nom::IResult<&str, IpAddr> {
        match i.parse::<IpAddr>() {
            Ok(addr) => IResult::Ok(("", addr)),
            Err(_) => IResult::Err(Err::Error(nom::error::Error {
                input: i,
                code: ErrorKind::Verify,
            })),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::net::{Ipv4Addr, Ipv6Addr};
        #[test]
        fn test_ip_address_parsing() {
            assert_eq!(
                parse_ip_address("127.0.0.1"),
                Ok(("", IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))))
            );
            assert_eq!(
                parse_ip_address("::1 -"),
                Ok((" -", IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))))
            );
            assert_eq!(
                parse_ip_address("127.0.0.1 - mary [09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12"),
                Ok((" - mary [09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12", IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))))
            );
            assert_eq!(
                parse_ip_address("-"),
                Err(Err::Error(nom::error::Error {
                    input: "-",
                    code: ErrorKind::Verify,
                }))
            );
        }
        #[test]
        fn not_whitespace_test() {
            assert_eq!(
                not_whitespace("before_whitespace after_whitespace"),
                Ok((" after_whitespace", "before_whitespace"))
            );
            assert_eq!(
                not_whitespace("before_tab\tafter_tab"),
                Ok(("\tafter_tab", "before_tab"))
            );
            assert_eq!(
                not_whitespace(" after_space"),
                Err(nom::Err::Error(nom::error::Error::new(
                    " after_space",
                    nom::error::ErrorKind::IsNot
                )))
            );
        }
    }
}
