220k req/s ~150 MB of Memory
500 req/s ~6-11KB

There's no guarantee of logs being sorted in the file, so while initally reading the file - each entry is matched against range of time `(now - refresh_interval)..(now)`, which adds some computational cost and you might experience some slow starts on big log files with 100k+ entries (well, depends on disk and CPU).

<h3>Things to improve</h3>

- [user-identifier](https://tools.ietf.org/html/rfc1413) is not really parsed as it is not usually used, it is assumed that it always has a value of "-". I'm not sure if it is really used in web-applications, but this parser will skip entries with non-empty identifier so far, assuming they are mailformed. TODO is to actually accept entries with full identifier according to RFC

- Error handling in Parser:

  - Right now it's not that obvious which part of a string is messed up, errors are propogated from nom library
  - A lot of errors are resulting in wrapped types (Err(Err(Err(...)))). Monads ftw, but amount of time I have for the assignemnt is not playing in my favour in combination to my knowledge about Monads, so I'll leave it for now.
  - I took a decision to replace unparsable things with "-" if text and "0" if digits, it is going to be considered later in the statistics part of the program
    - if failed to parse IP address => 0.0.0.0
    - If failed to parse user_id => "-"
    - If failed to parse response code => 0
    - If failed to parse date => 1970-01-01T00:00:00-00:00

- parse_response_code and parse_response_size functions are almost identical, they just return slightly different type. It probably is possible to refactor it into using generics, but I'd not risk my time for that at this moment
- File descriptor changes are not handled

parse\_\*\_helper_dash could be probably generalized with polymorthic datatypes and (Default trait)[https://doc.rust-lang.org/std/default/trait.Default.html], but again, at this moment it'd make it just more confusing.
