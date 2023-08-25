use crossterm::{
    cursor::{MoveToNextLine, MoveToPreviousLine},
    event::{read, Event, KeyCode, KeyEvent, KeyEventKind},
    style::Print,
    terminal::{self, Clear, ClearType},
    QueueableCommand,
};
use seahorse::{App, Context, Flag, FlagType};
use std::{
    env,
    io::{self, Write},
    str::FromStr,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};
use timers::{Hms, Timer};

const FLAG_NAME_COUNT_DOWN: &str = "count-down";

fn main() {
    let args: Vec<String> = env::args().collect();
    App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage("timers [OPTIONS]")
        .action(default_action)
        .flag(
            Flag::new(FLAG_NAME_COUNT_DOWN, FlagType::String)
                .description("<string> of [[hour:]minute:]second like 1:23:45 or 1:00 or 30")
                .alias("c"),
        )
        .run(args);
}

fn default_action(c: &Context) {
    if let Ok(time) = c.string_flag(FLAG_NAME_COUNT_DOWN) {
        let hms: Result<Hms, String> = Hms::from_str(&time);
        match hms {
            Err(s) => println!("{}", s),
            Ok(hms) => count_down_task(hms),
        };
    } else {
        stopwatch_task()
    }
}

fn spawn_stdin_channel() -> Receiver<KeyEvent> {
    let (tx, rx) = mpsc::channel::<KeyEvent>();
    thread::spawn(move || loop {
        let res: Result<Event, io::Error> = read();
        let res: Result<(), &str> = match res {
            Ok(e) => match e {
                Event::Key(key) => tx
                    .send(key)
                    .map_err(|_| "failed to communicate between threads"),
                _ => Ok(()),
            },
            Err(_) => Err("failed to read from standard input"),
        };
        if let Err(s) = res {
            terminal::disable_raw_mode().unwrap();
            println!("{}", s);
            println!("read task is killing");
            break;
        }
    });
    rx
}

fn count_down_task(hms: Hms) {
    println!("{}", hms);
}

fn stopwatch_task() {
    println!("stop at <Space> or <k> or <Esc>");
    println!("start");
    println!("exit at <q> or <Esc> or <Enter>");
    if terminal::enable_raw_mode().is_err() {
        println!("terminal can't change to raw mode");
        return;
    }
    let mut based_time: Instant = Instant::now();
    let mut saved_duration: Duration = Duration::ZERO;
    let mut timer: Timer = Timer::RUN;
    let stdin_channel: Receiver<KeyEvent> = spawn_stdin_channel();
    let exit_message: &str = loop {
        match stdin_channel.try_recv() {
            Ok(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                            break "finish!";
                        }
                        KeyCode::Char(' ') | KeyCode::Char('k') => match timer {
                            Timer::RUN => {
                                timer = Timer::STOP;
                                saved_duration += based_time.elapsed();
                                if print_hms(saved_duration, &timer).is_err() {
                                    break "stdout write error";
                                }
                            }
                            Timer::STOP => {
                                timer = Timer::RUN;
                                based_time = Instant::now();
                            }
                        },
                        _ => (),
                    }
                }
            }
            Err(TryRecvError::Empty) => {
                if timer == Timer::RUN {
                    let duration: Duration = saved_duration + based_time.elapsed();
                    if print_hms(duration, &timer).is_err() {
                        break "stdout write error";
                    }
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(TryRecvError::Disconnected) => {
                break "Channel disconnected";
            }
        }
    };
    terminal::disable_raw_mode().unwrap();
    println!("{}", exit_message);
}

fn print_hms(now: Duration, timer: &Timer) -> Result<(), io::Error> {
    let state = match timer {
        Timer::RUN => "run:  ",
        Timer::STOP => "stop: ",
    };
    io::stdout()
        .queue(Clear(ClearType::CurrentLine))
        .and_then(|o| o.queue(MoveToPreviousLine(2)))
        .and_then(|o| o.queue(Print(format!("{state} {}", Hms::from(now)))))
        .and_then(|o| o.queue(MoveToNextLine(2)))
        .and_then(|o| o.flush())
}
