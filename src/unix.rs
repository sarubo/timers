use std::sync::{Arc, Mutex};

use termios::{
    os::target::{ECHO, ICANON, TCSANOW},
    tcsetattr, Termios,
};

pub fn enable_raw_mode(raw_fd: i32) -> Result<(), String> {
    let r = Termios::from_fd(raw_fd); // RawFdの数字を知りたい。
    let Ok(mut t) = r else {
        return Err("Termios from fd is fail".to_owned());
    };
    *t.c_lflag &= !(ICANON | ECHO);
    tcsetattr(fd, TCSANOW, &mut m).map_err(|_| "enable raw mode is fail on tcsetattr".to_owned())
}

pub fn disable_raw_mode(mode: Arc<Mutex<Termios>>) -> Result<(), String> {
    let r = Termios::from_fd(raw_fd); // RawFdの数字を知りたい。
    let Ok(mut t) = r else {
        return Err("Termios from fd is fail".to_owned());
    };
    *t.c_lflag |= ICANON | ECHO;
    tcsetattr(fd, TCSANOW, &mut m).map_err(|_| "disable raw mode is fail on tcsetattr".to_owned())
}
