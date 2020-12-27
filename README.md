# clf-parser

clf-parser is a Rust application for dealing with [CLF](https://en.wikipedia.org/wiki/Common_Log_Format). Something that will help one to create information out of their raw log data.

## Foreword

I'm not a very experienced programmer so a careful reviewer might find some of the decisions I made in the code not optimal, cringe, or even disturbing. I expect things like that to be there indeed as I had to fight quite some time with Rust's compiler (it wouldn't always agree with my types, stubborn piece of software).

But at the same time, I want to say that I had fun doing this assignment, it gave me an opportunity for really good practice backed with good motivation and I learned a few things doing it, which is always a nice thing to happen, so I hope you'd find it nice at least to some extend.

## Installation

A binary build for `x86_64 GNU/Linux` is included in the root of the repository.

The software was only tested for Linux systems, and won't work on Windows, but might work on Mac.

To build on Mac one would need to [install Rust](https://www.rust-lang.org/tools/install) and then run

```bash
cargo build --release
```

The resulting binary will appear as `target/release/clf-parser`

## UI

![UI Demo](./demo.gif)

## Usage

To exit the application - press `q`

```
USAGE:
    clf-parser [FLAGS] [OPTIONS]

FLAGS:
        --follow-only    Only follow the newly added content to file, do not read previously
                         generated lines of code. Good for large files that are known for holding a
                         lot of old logs
        --help           Prints help information
    -V, --version        Prints version information

OPTIONS:
        --alert-interval <alert-interval>
            Alerts after this much seconds traffic being more than threshold on average, threshold
            is set by --alert-threshold [default: 20]

        --alert-threshold <alert-threshold>      threshold for the alert [default: 10]
    -f, --filename <filename>
            Path to the file to watch [default: /tmp/access.log]

    -h, --http-codes <http-codes>...
            HTTP response codes to reflect statistics on [default: 200,401,404,500,501,502,504]

    -r, --refresh-interval <refresh-interval>
            Sets an interval of refreshing the statistics in seconds, max 256 [default: 10]
```

Defaults are set to what is asked in the assignment text, but I find it rather convenient to use `clf-parser --refresh-interval 3 --follow-only` just to get data rendered faster

## Design Choices

### Rust

Why Rust? There are a few reasons for that:

- it is my freshest language. I've been studying it for quite some months now.
- I'm not an experienced programmer, developing code was not my day-to-day activity in the last few years, and I wanted to be protected from fatal rookie mistakes by the safety of Rust's type system, which worked out perfectly.
- It's [blazingly fast](https://benchmarksgame-team.pages.debian.net/benchmarksgame/performance/nbody.html) and I'd assume it is just important quality for real-time parsing software.

### General notes

The application runs a few threads that are synchronized through [channels](https://doc.rust-lang.org/std/sync/mpsc/fn.channel.html):

- Parser - reads file line-by-line and parses it to data
- Stats - a thread that aggregated data to information that we see in the UI
- Input - keyboard-input listening thread (To catch `q` is pressed and exit the application)
- UI rendering thread

Overall it works like this:

```
/tmp/access.log <-- watch/read --> Parser --> Stats -> UI

Input -> UI
```

All the components are decoupled through the channels, which would make it relatively painless to chip each component off and place it separately replacing channels with network.

### Parser

The parser is built using [parser combinator](https://en.wikipedia.org/wiki/Parser_combinator#:~:text=In%20computer%20programming%2C%20a%20parser,new%20parser%20as%20its%20output.) programming pattern and can be found in [src/parser_combinators.rs](src/parser_combinators.rs).

TL&DR parser combinator is just a bunch of functions that are capable of parsing one little piece of text combined in one big function.

Essentially one divides a problem of parsing a piece of text into many little problems and then combining solutions to this little problems to one big function. In my case it would be:

```rust
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
```

So, it's a function that you feed with a bunch of other functions that can parse very simple pieces of text and return the rest of the input until the input is exhausted. I find the code above quite self-explanatory.

Having all these little parser-functions leads to ease of testing as well.

A careful reader might wonder: why have I chosen parser combinator as a way of solving this exact problem? The CLF is quite a simple thing and regular `.split(" ")` could have been just enough, right? Well, there are a few reasons, my friends:

- It's a test assignment for a job interview in the end. One needs to demonstrate that they know something better than simple `.split(" ")`, even though it would perfectly work in this exact case. Knowing parses combinator would indicate that one would not struggle with parsing more complex than CLF data formats
- I happened to go through a [wonderful tutorial about parser combintors](https://bodil.lol/parser-combinators/) a few weeks ago and I was very excited about trying it on something real so I knew almost instantly how I'm going to do it exactly :P
- There's a parser-combinator state-of-art [Rust library](https://github.com/Geal/nom) and I wanted to get some experience with it particularly as it is an important and very powerful tool

### Stats

Stats thread is located in [src/lib.rs](src/lib.rs) and is mainly represented by the `collect_stats` function that receives log entries from the parser thread through looped `rx_logs.recv_timeout(recieve_timeout) ` and aggregates received data with simple arithmetics all over the body function.

There's a lot of ugly typecasting like `u64` -> `i64` -> `usize` going on. Well, I guess it's a price you pay for having statically-typed language as your tool.

### Input

Just a little function `keyboard_listener` in [src/lib.rs](src/lib.rs) that listens to button "q" to be pressed and sends an exit signal to UI if it happened

### UI

Built with [tui-rs](https://github.com/fdehau/tui-rs) library for dynamic visual effects. Essentially the only thing that's going on there is converting types received from the `Stats` thread into [tui-rs](https://github.com/fdehau/tui-rs) data structures and generating layout squares depending on the config.

## Performance

For generating dummy logs I used [flog](https://github.com/mingrammer/flog) utility.

On my system `flog` was able to write ~250k/s CLF entries at the top to the same file being run as `./flog -l >> /tmp/access.log`, and `clf-parser` was perfectly capable of handling this load, I didn't test any further considering that parsing, analyzing and reflecting stats on `~250k/s` entries is more than enough as the first iteration of this software, but I'd suggest it's not a limit.

Memory consumption is quite static if the log rate doesn't change, but also depends on the configuration you pass to the binary:

- 220000 req/s ~100-150MB
- 500 req/s ~6-11KB

Memory consumption doesn't grow linearly but memory profiling goes far beyond this project initial goal, overall I find these values quite satisfying.

## Testing

Tests only cover the parser part of the program, to run them:

```
cargo test
```

Initial problem statement required to test alert logic, but I found it much more important to dedicate my limited time to things that has bigger probability of failure, such as parser, so the only unit-tests you'd find here are those that test all the possible parsers in [src/parser_combinators.rs](src/parser_combinators.rs), and in the same time, alerting logic is just a few lines of simple arithmetics and they are not so essentially to be tested as parsing part:

```rust
                let avg_val_in_alert_interval = alerting_buffer
                    .collect::<Vec<u64>>()
                    .into_iter()
                    .sum::<u64>()
                    / (max_alerting_buffer_len * refresh_interval as usize) as u64;
                if avg_val_in_alert_interval >= alert_threshold as u64 {
                    if let RenderMessage::UI(ref mut message) = render_message {
                        message.threshold_reached = true;
                        message.avg_within_alert_interval = avg_val_in_alert_interval;
                    }
                };
```

## Road Map

- General

  - There's no guarantee of logs being sorted in the file, so while initially reading the file - each entry is matched against a range of time `(now - refresh_interval)..(now)`, which adds some computational cost and you might experience some slow starts on big log files with 100k+ entries (well, depends on disk and CPU).
  - For the same reason it might be a good idea to implement reverse-read so the parser wouldn't bother to parse stale logs

  - Handle file descriptor change
  - Split `lib.rs` into two different components responsible for reading file/parsing and analyzing data
  - Refactor `collect_stats` as it happened to be too big and noisy in the end
  - Unit tests for `collect_stats` function
  - Refactor error handling when the file can not be opened, it breaks terminal because of incorrect exit from UI-thread
  - Add color scheme-configs

- Parser

  - Improve error handling in Parser:

    - Right now it's not that obvious which part of a string is messed up, errors are propagated from nom library
    - A lot of errors are resulting in wrapped types (Err(Err(Err(...)))). Monads ftw, but the amount of time I have for the assignment is not playing in my favor in combination with my knowledge about Monads, so I'll leave it for now.
    - I decided to replace unparsable things with "-" if text and "0" if digits, it is going to be considered later in the statistics part of the program, it might be a good idea just to drop these

      - if failed to parse IP address => 0.0.0.0
      - If failed to parse user_id => "-"
      - If failed to parse response code => 0
      - If failed to parse date => 1970-01-01T00:00:00-00:00

    - `parse_response_code` and `parse_response_size` functions are almost identical, they just return slightly different type. It probably is possible to refactor it into using generics, but I'd not risk my time for that at this moment

## License

[MIT](https://choosealicense.com/licenses/mit/)
