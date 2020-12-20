use chrono::{DateTime, FixedOffset};
use std::fmt;
use std::fmt::Display;
use std::net::IpAddr;

#[derive(Debug, PartialEq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub protocol: String,
}

impl Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method, self.path)
    }
}
// Example of the time log:
// 127.0.0.1 user-identifier frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326
#[derive(PartialEq, Debug)]
pub struct LogEntry {
    pub ip_address: IpAddr,
    pub identifier: String,
    pub user_id: String,
    pub timestamp: DateTime<FixedOffset>,
    pub request: Request,
    pub response_code: u16,
    pub response_size: u128,
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
            self.identifier,
            self.user_id,
            self.timestamp,
            self.request,
            self.response_code,
            self.response_size
        )
    }
}

#[allow(dead_code)]
pub mod parsers {
    use super::*;
    use std::net::Ipv4Addr;

    fn is_not_whitespace(i: &str) -> nom::IResult<&str, &str> {
        nom::bytes::complete::is_not(" \t")(i)
    }

    fn is_whitespace(i: &str) -> nom::IResult<&str, &str> {
        nom::bytes::complete::is_a(" \t")(i)
    }

    fn parse_ip_address(i: &str) -> nom::IResult<&str, IpAddr> {
        fn ip_parser_helper(i: &str) -> nom::IResult<&str, IpAddr> {
            match i.parse::<IpAddr>() {
                Ok(addr) => Ok(("", addr)),
                Err(_) => Ok(("", IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)))),
            }
        }

        nom::combinator::map_parser(is_not_whitespace, ip_parser_helper)(i)
    }

    fn parse_timestamp(i: &str) -> nom::IResult<&str, DateTime<FixedOffset>> {
        fn parse_timestamp_helper(i: &str) -> nom::IResult<&str, DateTime<FixedOffset>> {
            match DateTime::parse_from_str(i, "%d/%b/%Y:%H:%M:%S %z") {
                Ok(date) => Ok(("", date)),
                Err(_) => Ok((
                    "]",
                    DateTime::parse_from_rfc3339("1970-01-01T00:00:00-00:00").unwrap(),
                )),
            }
        }
        fn parse_timestamp_helper_dash(i: &str) -> nom::IResult<&str, DateTime<FixedOffset>> {
            match nom::bytes::complete::tag("-")(i) {
                Ok((rest, _)) => Ok((
                    rest,
                    DateTime::parse_from_rfc3339("1970-01-01T00:00:00-00:00").unwrap(),
                )),
                Err(err) => Err(err),
            }
        }
        nom::branch::alt((
            nom::sequence::delimited(
                nom::character::complete::char('['),
                nom::combinator::map_parser(
                    nom::bytes::complete::take_until("]"),
                    parse_timestamp_helper,
                ),
                nom::character::complete::char(']'),
            ),
            parse_timestamp_helper_dash,
        ))(i)
    }

    fn parse_request(i: &str) -> nom::IResult<&str, Request> {
        fn parse_request_helper_dash(i: &str) -> nom::IResult<&str, Request> {
            match nom::bytes::complete::tag("-")(i) {
                Ok((rest, _)) => Ok((
                    rest,
                    Request {
                        method: String::from("-"),
                        path: String::from("-"),
                        protocol: String::from("-"),
                    },
                )),
                Err(err) => Err(err),
            }
        }
        fn parse_request_helper(i: &str) -> nom::IResult<&str, Request> {
            match nom::sequence::delimited(
                nom::character::complete::char('"'),
                nom::sequence::tuple((
                    is_not_whitespace,
                    is_whitespace,
                    is_not_whitespace,
                    is_whitespace,
                    nom::bytes::complete::take_until("\""),
                )),
                nom::character::complete::char('"'),
            )(i)
            {
                Ok((rest, (result1, _, result2, _, result3))) => Ok((
                    rest,
                    Request {
                        method: String::from(result1),
                        path: String::from(result2),
                        protocol: String::from(result3),
                    },
                )),
                Err(err) => Err(err),
            }
        }
        nom::branch::alt((parse_request_helper_dash, parse_request_helper))(i)
    }

    fn parse_response_code(i: &str) -> nom::IResult<&str, u16> {
        fn parse_response_code_helper_dash(i: &str) -> nom::IResult<&str, u16> {
            match nom::character::complete::char('-')(i) {
                Err(err) => Err(err),
                Ok((rest, _)) => Ok((rest, 0)),
            }
        }
        fn parse_response_code_helper(i: &str) -> nom::IResult<&str, u16> {
            match nom::character::complete::digit1(i) {
                Ok((rest, code)) => match code.parse::<u16>() {
                    Ok(code) => Ok((rest, code)),
                    Err(_) => Ok((rest, 0)),
                },
                Err(err) => Err(err),
            }
        }
        nom::branch::alt((parse_response_code_helper_dash, parse_response_code_helper))(i)
    }

    fn parse_response_size(i: &str) -> nom::IResult<&str, u128> {
        fn parse_response_size_helper_dash(i: &str) -> nom::IResult<&str, u128> {
            match nom::character::complete::char('-')(i) {
                Err(err) => Err(err),
                Ok((rest, _)) => Ok((rest, 0)),
            }
        }
        fn parse_response_size_helper(i: &str) -> nom::IResult<&str, u128> {
            match nom::character::complete::digit1(i) {
                Ok((rest, code)) => match code.parse::<u128>() {
                    Ok(code) => Ok((rest, code)),
                    Err(_) => Ok((rest, 0)),
                },
                Err(err) => Err(err),
            }
        }
        nom::branch::alt((parse_response_size_helper_dash, parse_response_size_helper))(i)
    }
    pub fn parse_log_entry(i: &str) -> Result<LogEntry, String> {
        match nom::combinator::all_consuming(nom::sequence::tuple((
            // 127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326
            parse_ip_address,    // 127.0.0.1
            is_whitespace,       // " "
            parse_identifier,    // "-"
            is_whitespace,       // " "
            is_not_whitespace,   // "frank"
            is_whitespace,       // " "
            parse_timestamp,     // [10/Oct/2000:13:55:36 -0700]
            is_whitespace,       // " "
            parse_request,       // "GET /apache_pb.gif HTTP/1.0"
            is_whitespace,       // " "
            parse_response_code, // "200"
            is_whitespace,       // " "
            parse_response_size, // "2326"
        )))(i)
        {
            Ok((
                _,
                (
                    ip_address,
                    _,
                    identifier,
                    _,
                    user_id,
                    _,
                    timestamp,
                    _,
                    request,
                    _,
                    response_code,
                    _,
                    response_size,
                ),
            )) => Ok(LogEntry {
                ip_address: ip_address,
                user_id: String::from(user_id),
                identifier: identifier,
                timestamp: timestamp,
                request: request,
                response_code: response_code,
                response_size: response_size,
            }),
            Err(err) => Err(format!(
                "Could not parse log entry: {} because of error: {}",
                i, err
            )),
        }
    }
    //TODO: Implement RFC1413 instead of just -
    fn parse_identifier(i: &str) -> nom::IResult<&str, String> {
        match nom::character::complete::char('-')(i) {
            Ok((rest, user_id)) => Ok((rest, String::from(user_id))),
            Err(err) => Err(err),
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
                parse_ip_address("192.168.0.1 - mary [09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12"),
                Ok((" - mary [09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12", IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))))
            );
            assert_eq!(
                parse_ip_address("-"),
                Ok(("", IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))))
            );
        }
        #[test]
        fn identifier_parser_test() {
            assert_eq!(
                parse_identifier(
                    "- mary [09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12"
                ),
                Ok((
                    " mary [09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12",
                    String::from("-")
                ))
            );
        }
        #[test]
        fn is_not_whitespace_test() {
            assert_eq!(
                is_not_whitespace(
                    "mary [09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12"
                ),
                Ok((
                    " [09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12",
                    "mary"
                ))
            );
            assert_eq!(
                is_not_whitespace(
                    "- [09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12"
                ),
                Ok((
                    " [09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12",
                    "-"
                ))
            );
        }
        #[test]
        fn parse_timestamp_test() {
            assert_eq!(
                parse_timestamp("[09/May/2018:16:00:42 +0000] \"POST /api/user HTTP/1.0\" 503 12"),
                Ok((
                    " \"POST /api/user HTTP/1.0\" 503 12",
                    DateTime::parse_from_str("09/May/2018:16:00:42 +0000", "%d/%b/%Y:%H:%M:%S %z")
                        .unwrap()
                ))
            );
            assert_eq!(
                parse_timestamp("- \"POST /api/user HTTP/1.0\" 503 12"),
                Ok((
                    " \"POST /api/user HTTP/1.0\" 503 12",
                    DateTime::parse_from_rfc3339("1970-01-01T00:00:00-00:00").unwrap()
                ))
            );
        }
        #[test]
        fn parse_request_test() {
            assert_eq!(
                parse_request("\"POST /api/user HTTP/1.0\" 503 12"),
                Ok((
                    " 503 12",
                    Request {
                        method: String::from("POST"),
                        path: String::from("/api/user"),
                        protocol: String::from("HTTP/1.0")
                    }
                ))
            );
            assert_eq!(
                parse_request("- 503 12"),
                Ok((
                    " 503 12",
                    Request {
                        method: String::from("-"),
                        path: String::from("-"),
                        protocol: String::from("-")
                    }
                ))
            );
        }
        #[test]
        fn parse_log_entry_test() {
            assert_eq!(
                parse_log_entry("127.0.0.1 - james [09/May/2018:16:00:39 +0000] \"GET /report HTTP/1.0\" 200 123"),
                Ok(LogEntry{
                    ip_address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    identifier: String::from("-"),
                    user_id: String::from("james"),
                    timestamp: DateTime::parse_from_str("09/May/2018:16:00:39 +0000", "%d/%b/%Y:%H:%M:%S %z")
                    .unwrap(),
                    request: Request {
                        method: String::from("GET"),
                        path: String::from("/report"),
                        protocol: String::from("HTTP/1.0")
                    },
                    response_code: 200,
                    response_size: 123

                })
            );
            assert_eq!(
                parse_log_entry(
                    "- - james [09/May/2018:16:00:39 +0000] \"GET /report HTTP/1.0\" 200 123"
                ),
                Ok(LogEntry {
                    ip_address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    identifier: String::from("-"),
                    user_id: String::from("james"),
                    timestamp: DateTime::parse_from_str(
                        "09/May/2018:16:00:39 +0000",
                        "%d/%b/%Y:%H:%M:%S %z"
                    )
                    .unwrap(),
                    request: Request {
                        method: String::from("GET"),
                        path: String::from("/report"),
                        protocol: String::from("HTTP/1.0")
                    },
                    response_code: 200,
                    response_size: 123
                })
            );
            assert_eq!(
                parse_log_entry(
                    "- - - [09/May/2018:16:00:39 +0000] \"GET /report HTTP/1.0\" 200 123"
                ),
                Ok(LogEntry {
                    ip_address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    identifier: String::from("-"),
                    user_id: String::from("-"),
                    timestamp: DateTime::parse_from_str(
                        "09/May/2018:16:00:39 +0000",
                        "%d/%b/%Y:%H:%M:%S %z"
                    )
                    .unwrap(),
                    request: Request {
                        method: String::from("GET"),
                        path: String::from("/report"),
                        protocol: String::from("HTTP/1.0")
                    },
                    response_code: 200,
                    response_size: 123
                })
            );
            assert_eq!(
                parse_log_entry("- - - - - 200 123"),
                Ok(LogEntry {
                    ip_address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    identifier: String::from("-"),
                    user_id: String::from("-"),
                    timestamp: DateTime::parse_from_rfc3339("1970-01-01T00:00:00-00:00").unwrap(),
                    request: Request {
                        method: String::from("-"),
                        path: String::from("-"),
                        protocol: String::from("-")
                    },
                    response_code: 200,
                    response_size: 123
                })
            );
            assert_eq!(
                parse_log_entry("- - - - - - 123"),
                Ok(LogEntry {
                    ip_address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    identifier: String::from("-"),
                    user_id: String::from("-"),
                    timestamp: DateTime::parse_from_rfc3339("1970-01-01T00:00:00-00:00").unwrap(),
                    request: Request {
                        method: String::from("-"),
                        path: String::from("-"),
                        protocol: String::from("-")
                    },
                    response_code: 0,
                    response_size: 123
                })
            );
            assert_eq!(
                parse_log_entry("100.100.100.100 - - - - - 123"),
                Ok(LogEntry {
                    ip_address: IpAddr::V4(Ipv4Addr::new(100, 100, 100, 100)),
                    identifier: String::from("-"),
                    user_id: String::from("-"),
                    timestamp: DateTime::parse_from_rfc3339("1970-01-01T00:00:00-00:00").unwrap(),
                    request: Request {
                        method: String::from("-"),
                        path: String::from("-"),
                        protocol: String::from("-")
                    },
                    response_code: 0,
                    response_size: 123
                })
            );
            assert_eq!(
                parse_log_entry("- - - - - - -"),
                Ok(LogEntry {
                    ip_address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    identifier: String::from("-"),
                    user_id: String::from("-"),
                    timestamp: DateTime::parse_from_rfc3339("1970-01-01T00:00:00-00:00").unwrap(),
                    request: Request {
                        method: String::from("-"),
                        path: String::from("-"),
                        protocol: String::from("-")
                    },
                    response_code: 0,
                    response_size: 0
                })
            );
        }
    }
}
