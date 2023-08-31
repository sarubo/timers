use seahorse::{App, Context};
use std::{
    env,
    io::{self, stdin, Read, Write},
    str::FromStr,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};
use timers::{disable_raw_mode, enable_raw_mode, Hms, Timer};

fn main() {
    // TODO: win ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT;
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

fn spawn_stdin_channel() -> Receiver<char> {
    let (tx, rx) = mpsc::channel::<char>();
    thread::spawn(move || loop {
        let buf: Result<[u8; 1], &str> = {
            let mut buf: [u8; 1] = [0];
            let mut f = stdin().lock();
            f.read(&mut buf).map_err(|_| "read is fail").map(|_| buf)
        };
        let res: Result<(), &str> = buf
            .and_then(|us| us.first().map(|u| u.to_owned()).ok_or("read char is None"))
            .and_then(|u| char::try_from(u).map_err(|_| "read char is invalid"))
            .and_then(|c| {
                tx.send(c)
                    .map_err(|_| "failed to communicate between threads")
            });
        if let Err(s) = res {
            disable_raw_mode(0).unwrap();
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
    if let Err(s) = enable_raw_mode(0) {
        println!("terminal can't change to raw mode\n{}", s);
        return;
    }
    let mut based_time: Instant = Instant::now();
    let mut saved_duration: Duration = match hms {
        Some(hms) => hms.to_duration(),
        None => Duration::ZERO,
    };
    let mut timer: Timer = Timer::RUN;
    let stdin_channel: Receiver<char> = spawn_stdin_channel();
    let stdin_channel = stdin_channel;
    let exit_message: String = loop {
        match stdin_channel.try_recv() {
            Ok(key) => match key {
                'q' | '\r' | '\n' => {
                    break "finish!".to_owned();
                }
                ' ' | 'k' => match timer {
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
            },
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
    if let Err(s) = disable_raw_mode(0) {
        println!("terminal can't change to raw mode\n{}", s);
        return;
    }
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
    let out: Vec<String> = vec![
        delete_line(),
        up_to_head(1),
        delete_line(),
        format!("{state} {}", Hms::from(now)),
        down_to_head(1),
    ];
    let mut lock: io::StdoutLock<'_> = io::stdout().lock();
    write!(&mut lock, "{}", out.join(""))?;
    lock.flush()
}

fn delete_line() -> String {
    "\x1b[2K".to_owned()
}

fn down_to_head(num: u8) -> String {
    format!("\x1b[{}E", num)
}

fn up_to_head(num: u8) -> String {
    format!("\x1b[{}F", num)
}
