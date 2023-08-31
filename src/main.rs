use crossterm::{
    cursor::{MoveToNextLine, MoveToPreviousLine},
    event::{read, Event, KeyCode, KeyEvent, KeyEventKind},
    style::Print,
    terminal::{self, Clear, ClearType},
    QueueableCommand,
};
use seahorse::{App, Context};
use std::{
    env,
    io::{self, Write},
    str::FromStr,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};
use timers::{Hms, Timer};

fn main() {
    let args: Vec<String> = env::args().collect();
    App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .usage("timers [OPTIONS]")
        .command(
            seahorse::Command::new("countdown")
                .alias("c")
                .description("<string> of [[hour:]minute:]second like 1:23:45 or 1:00 or 30")
                .action(countdown_action),
        )
        .command(
            seahorse::Command::new("stopwatch")
                .alias("s")
                .description("start stop watch")
                .action(|_| loop_task(None)),
        )
        .action(|c| c.help())
        .run(args);
}

fn countdown_action(c: &Context) {
    if c.args.is_empty() {
        println!("need <string> of [[hour:]minute:]second");
    } else if let Some(s) = c.args.get(0) {
        match Hms::from_str(s) {
            Ok(hms) => loop_task(Some(hms)),
            Err(s) => println!("{s}"),
        }
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
            terminal::disable_raw_mode()
                .unwrap_or_else(|_| println!("terminal can't change to raw mode"));
            println!("{}", s);
            println!("read task is killing");
            break;
        }
    });
    rx
}

fn loop_task(hms: Option<Hms>) {
    println!("stop at <k> or <Space>");
    println!("exit at <q> or <Esc> or <Enter>");
    println!("start");
    if terminal::enable_raw_mode().is_err() {
        println!("terminal can't change to raw mode");
        return;
    }
    let mut based_time: Instant = Instant::now();
    let mut saved_duration: Duration = match hms {
        Some(hms) => hms.to_duration(),
        None => Duration::ZERO,
    };
    let mut timer: Timer = Timer::RUN;
    let stdin_channel: Receiver<KeyEvent> = spawn_stdin_channel();
    let exit_message: String = loop {
        match stdin_channel.try_recv() {
            Ok(key) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                        break "finish!".to_owned();
                    }
                    KeyCode::Char(' ') | KeyCode::Char('k') => match timer {
                        Timer::RUN => {
                            timer = Timer::STOP;
                            match calculate_and_print_duraition(
                                &hms,
                                &saved_duration,
                                &based_time,
                                &timer,
                            ) {
                                Ok(d) => saved_duration = d,
                                Err(s) => break s,
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
            Err(TryRecvError::Empty) => {
                if timer == Timer::RUN {
                    if let Err(s) =
                        calculate_and_print_duraition(&hms, &saved_duration, &based_time, &timer)
                    {
                        break s;
                    }
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(TryRecvError::Disconnected) => {
                break "Channel disconnected".to_owned();
            }
        }
    };
    terminal::disable_raw_mode().unwrap_or_else(|_| println!("terminal can't change to raw mode"));
    println!("{}", exit_message);
}

fn calculate_and_print_duraition(
    hms: &Option<Hms>,
    saved_duration: &Duration,
    based_time: &Instant,
    timer: &Timer,
) -> Result<Duration, String> {
    match hms {
        Some(_) => {
            let subed = sub_duration(saved_duration, based_time);
            countdown_print(subed, timer)
        }
        None => {
            let added = add_duration(saved_duration, based_time)?;
            stopwatch_print(added, timer)
        }
    }
}

fn sub_duration(saved_duration: &Duration, based_time: &Instant) -> Duration {
    saved_duration
        .checked_sub(based_time.elapsed())
        .unwrap_or(Duration::ZERO)
}

fn add_duration(saved_duration: &Duration, based_time: &Instant) -> Result<Duration, String> {
    saved_duration
        .checked_add(based_time.elapsed())
        .ok_or("time is overflow".to_owned())
}

fn countdown_print(saved_duration: Duration, timer: &Timer) -> Result<Duration, String> {
    if print_hms(saved_duration, timer).is_err() {
        Err("stdout write error".to_owned())
    } else if saved_duration.is_zero() {
        Err("finish!".to_owned())
    } else {
        Ok(saved_duration)
    }
}

fn stopwatch_print(saved_duration: Duration, timer: &Timer) -> Result<Duration, String> {
    if print_hms(saved_duration, timer).is_err() {
        Err("stdout write error".to_owned())
    } else {
        Ok(saved_duration)
    }
}

fn print_hms(now: Duration, timer: &Timer) -> Result<(), io::Error> {
    let state = match timer {
        Timer::RUN => "run:  ",
        Timer::STOP => "stop: ",
    };
    io::stdout()
        .queue(Clear(ClearType::CurrentLine))
        .and_then(|o| o.queue(MoveToPreviousLine(1)))
        .and_then(|o| o.queue(Print(format!("{state} {}", Hms::from(now)))))
        .and_then(|o| o.queue(MoveToNextLine(1)))
        .and_then(|o| o.flush())
}
