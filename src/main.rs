use std::io::{self, Write};
use std::num::ParseIntError;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;

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
    /// WIP: [[hour] minute] second [[[subsecond]]]
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

fn delete_lf(s: String) -> String {
    let mut s = s;
    s.remove(s.len() - 1);
    s
}

#[derive(Debug, Clone, Copy)]
struct Hms {
    hour: u64,
    min: u64,
    sec: u64,
    subsec: u32
}

impl FromStr for Hms {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut hms: Vec<_> = s.split(' ').collect();
        hms.reverse();
        let mut i = hms.into_iter();
        let subsec = if i.len() >= 4 {
            i.next().unwrap_or("0").parse::<u32>()?
        } else {
            0_u32
        };
        let sec = i.next().unwrap_or("0").parse::<u64>()?;
        let min = i.next().unwrap_or("0").parse::<u64>()?;
        let hour = i.next().unwrap_or("0").parse::<u64>()?;
        if min < 60 && sec < 60 && subsec < 10 {
            panic!("You must follow min < 60 and sec < 60 and subsec < 10")
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
        Hms { hour, min, sec, subsec }
    }
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

fn main() {
    let _ = Args::parse();
    println!("stop at <Enter>", );

    println!("start");
    let mut based_time = Instant::now();
    let mut saved_duration = Duration::new(0, 0);
    let mut timer: Timer = Timer::RUN;

    let stdin_channel = spawn_stdin_channel();
    loop {
        match stdin_channel.try_recv() {
            Ok(key) => {
                print!("exit at \"q\"");
                up_to_head(1);
                if key == "\n" {
                    match timer {
                        Timer::STOP => {
                            timer = Timer::RUN;
                            based_time = Instant::now();
                        },
                        Timer::RUN => {
                            timer = Timer::STOP;
                            saved_duration = add_duration(saved_duration, based_time);
                            let hms = Hms::from(saved_duration);
                            print_hms(hms);
                        },
                    }
                }
                let key = delete_lf(key);
                if key == "q" {
                    delete_line(Delete::ALL);
                    println!("ok");
                    delete_line(Delete::ALL);
                    io::stdout().flush().unwrap();
                    break;
                }
                io::stdout().flush().unwrap();
            },
            Err(TryRecvError::Empty) => {
                if timer == Timer::RUN {
                    let duration = add_duration(saved_duration, based_time);
                    let hms = Hms::from(duration);
                    print_hms(hms);
                };
            },
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
        }
        sleep(100);
    }
}
