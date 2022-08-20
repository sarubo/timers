use core::fmt;
use std::fmt::Debug;
use std::io::{self, Write};
use std::num::ParseIntError;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;
use clap::builder::ValueParserFactory;

fn spawn_stdin_channel() -> Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        io::stdout().flush().unwrap();
        tx.send(buffer).unwrap();
    });
    rx
}

fn sleep(millis: u64) {
    let duration = Duration::from_millis(millis);
    thread::sleep(duration);
}

/// lightweight stopwatch and timer
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// [[hour]:minute]:second
    #[clap(short, long, value_parser)]
    count_down: Option<Hms>,
}

enum Delete {
    LEFT,
    RIGHT,
    ALL
}

impl Delete {
    fn val(&self) -> u8 {
        match self {
            Delete::LEFT => 0,
            Delete::RIGHT => 1,
            Delete::ALL => 2,
        }
    }
}
#[derive(PartialEq)]
enum Timer {
    RUN,
    STOP,
}

fn delete_line(at: Delete) { print!("\x1b[{}K", at.val()); }

fn down_to_head(num: u8)  { print!("\x1b[{}E", num); }

fn up_to_head(num: u8) { print!("\x1b[{}F", num); }

fn add_duration(old: Duration, now: Instant) -> Duration {
    old
        .checked_add(now.elapsed())
        .unwrap_or(Duration::new(0, 0))
}

fn sub_duration(saved_duration: Duration, based_time: Instant) -> Duration {
    match saved_duration.checked_sub(based_time.elapsed()) {
        Some(subed_duration) => subed_duration,
        None => Duration::ZERO,
    }
}

fn delete_lf(s: String) -> String {
    let mut s = s;
    s.remove(s.len() - 1);
    s
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParseError {
    Int(ParseIntError),
    Condition(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Int(e) => std::fmt::Display::fmt(&e, f),
            ParseError::Condition(s) => std::fmt::Display::fmt(&s, f),
        }
    }
}

impl From<ParseIntError> for ParseError {
    fn from(e: ParseIntError) -> Self {
        Self::Int(e)
    }
}

#[derive(Debug, Clone, Copy)]
struct Hms {
    hour: u64,
    min: u64,
    sec: u64,
    subsec: u32
}

impl FromStr for Hms {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut hms: Vec<_> = s.split(':').collect();
        hms.reverse();
        let hms_len = hms.len();
        let mut i = hms.into_iter();
        let subsec = 0_u32;
        let sec = i.next().unwrap_or("0").parse::<u64>()?;
        let min = i.next().unwrap_or("0").parse::<u64>()?;
        let hour = i.next().unwrap_or("0").parse::<u64>()?;
        if min >= 60 || sec >= 60 || subsec >= 10 {
            Err(ParseError::Condition("You must follow min < 60 and sec < 60 and subsec < 10".to_owned()))
        } else if hms_len > 3 {
            Err(ParseError::Condition("You can only use hour, min and sec".to_owned()))
        } else {
            Ok(Self { hour, min, sec, subsec })
        }
    }
}

impl From<Duration> for Hms {
    fn from(duration: Duration) -> Self {
        let big_sec = duration.as_secs();
        let hour = big_sec / 60 / 60;
        let big_sec = big_sec - hour * 60 * 60;
        let min = big_sec / 60;
        let sec = big_sec - min * 60;
        let subsec = duration.subsec_nanos() / 100_000_000;
        Self { hour, min, sec, subsec }
    }
}

impl ValueParserFactory for Hms {
    type Parser = HmsParser;

    fn value_parser() -> Self::Parser {
        HmsParser {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HmsParser;

impl clap::builder::TypedValueParser for HmsParser {
    type Value = Hms;
    fn parse_ref(
        &self,
        cmd: &clap::Command,
        opt: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        match value.to_str() {
            Some(s) => {
                let r = Hms::from_str(s);
                r.map_or_else(|e| {
                    let mut s = format!(
                        "Invalid value {} for '{}':\n",
                        value.to_str().unwrap_or("<value>"),
                        opt.map(|a| a.to_string()).unwrap_or_default()
                    );
                    let message = match e {
                        ParseError::Int(e) => e.to_string(),
                        ParseError::Condition(s) => s,
                    };
                    s.push_str(&message);
                    let mut cmd = cmd.clone();
                    Err(cmd.error(clap::ErrorKind::InvalidValue, s))
                }, |h| Ok(h))
            },
            None => panic!("OsStr is bad"),
        }
    }
}

fn hms_to_duration(hms: Hms) -> Duration {
    let secs = hms.hour * 60 * 60 + hms.min * 60 + hms.sec;
    let nanos = hms.subsec * 1_000_000;
    Duration::new(secs, nanos)
}

fn print_hms(hms: Hms) {
    delete_line(Delete::ALL);
    up_to_head(1);
    delete_line(Delete::ALL);
    print!("{}:{:02}:{:02}.{:01}", hms.hour, hms.min, hms.sec, hms.subsec);
    down_to_head(1);
    delete_line(Delete::ALL);
    io::stdout().flush().unwrap();
}

fn print_exit() {
    delete_line(Delete::ALL);
    println!("ok");
    delete_line(Delete::ALL);
    io::stdout().flush().unwrap();
}

fn stopwatch_task() {
    println!("stop at <Enter>", );
    println!("start");
    print!("exit at \"q\"");
    up_to_head(1);
    io::stdout().flush().unwrap();
    let mut based_time = Instant::now();
    let mut saved_duration = Duration::ZERO;
    let mut timer: Timer = Timer::RUN;

    let stdin_channel = spawn_stdin_channel();
    loop {
        match stdin_channel.try_recv() {
            Ok(key) => {
                print!("exit at \"q\"");
                up_to_head(1);
                io::stdout().flush().unwrap();
                if key == "\n" {
                    match timer {
                        Timer::STOP => {
                            timer = Timer::RUN;
                            based_time = Instant::now();
                        },
                        Timer::RUN => {
                            timer = Timer::STOP;
                            saved_duration = add_duration(saved_duration, based_time);
                            print_hms(Hms::from(saved_duration));
                        },
                    }
                }
                let key = delete_lf(key);
                if key == "q" {
                    print_exit();
                    break;
                }
            },
            Err(TryRecvError::Empty) => {
                if timer == Timer::RUN {
                    let duration = add_duration(saved_duration, based_time);
                    print_hms(Hms::from(duration));
                }
            },
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
        }
        sleep(100);
    }
}

fn count_down_task(hms: Hms) {
    println!("stop at <Enter>", );
    println!("start");
    print!("exit at \"q\"");
    up_to_head(1);
    io::stdout().flush().unwrap();

    let mut based_time = Instant::now();
    let mut saved_duration = hms_to_duration(hms);
    let mut timer: Timer = Timer::RUN;

    let stdin_channel = spawn_stdin_channel();
    loop {
        match stdin_channel.try_recv() {
            Ok(key) => {
                print!("exit at \"q\"");
                up_to_head(1);
                io::stdout().flush().unwrap();
                if key == "\n" {
                    match timer {
                        Timer::STOP => {
                            timer = Timer::RUN;
                            based_time = Instant::now();
                        },
                        Timer::RUN => {
                            timer = Timer::STOP;
                            saved_duration = sub_duration(saved_duration, based_time);
                            print_hms(Hms::from(saved_duration));
                            if saved_duration.is_zero() {
                                break;
                            }
                        },
                    }
                }
                let key = delete_lf(key);
                if key == "q" {
                    print_exit();
                    break;
                }
            },
            Err(TryRecvError::Empty) => {
                if timer == Timer::RUN {
                    let duration = sub_duration(saved_duration, based_time);
                    print_hms(Hms::from(duration));
                    if duration.is_zero() {
                        break;
                    }
                };
            },
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
        }
        sleep(100);
    }
    println!("finish");
}

fn main() {
    let args = Args::parse();
    if let Some(hms) = args.count_down {
        count_down_task(hms);
    } else {
        stopwatch_task();
    }
}
