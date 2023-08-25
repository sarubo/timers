use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct Hms {
    hour: u64,
    min: u64,
    sec: u64,
    subsec: u32,
}

impl Hms {
    pub fn to_duration(&self) -> Duration {
        let secs = self.hour * 60 * 60 + self.min * 60 + self.sec;
        let nanos = self.subsec * 1_000_000;
        Duration::new(secs, nanos)
    }
}

impl FromStr for Hms {
    type Err = String;

    fn from_str(time: &str) -> Result<Self, Self::Err> {
        let parsed: Vec<_> = time.split(':').collect();
        let len: usize = parsed.len();
        let filtered: Vec<_> = parsed
            .into_iter()
            .filter_map(|s: &str| s.parse::<u64>().ok())
            .collect();
        if len != filtered.len() {
            return Err("There is a non-numeric input".to_owned());
        } else if !(1..=3).contains(&len) {
            return Err("You must follow [[hour:]minute:]second".to_owned());
        }
        let mut i = filtered.into_iter().rev();
        let subsec = 0_u32;
        let sec = i.next().unwrap_or(0);
        let min = i.next().unwrap_or(0);
        let hour = i.next().unwrap_or(0);
        if min >= 60 || sec >= 60 || subsec >= 10 {
            Err("You must follow min < 60 and sec < 60".to_owned())
        } else {
            Ok(Self {
                hour,
                min,
                sec,
                subsec,
            })
        }
    }
}

impl From<Duration> for Hms {
    fn from(duration: Duration) -> Self {
        let mut sec = duration.as_secs();
        let hour = sec / 60 / 60;
        sec -= hour * 60 * 60;
        let min = sec / 60;
        sec -= min * 60;
        let subsec = duration.subsec_nanos() / 100_000_000;
        Self {
            hour,
            min,
            sec,
            subsec,
        }
    }
}

impl Display for Hms {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{:02}:{:02}.{:01}",
            self.hour, self.min, self.sec, self.subsec
        )
    }
}

#[derive(PartialEq)]
pub enum Timer {
    RUN,
    STOP,
}
