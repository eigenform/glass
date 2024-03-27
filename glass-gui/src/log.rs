
use chrono::{Utc, Local, DateTime, Datelike, Timelike};

use crate::ipc::CameraMessage;

pub struct LogEntry { 
    time: DateTime<Local>,
    event: LogEvent,
}
impl LogEntry { 
    pub fn new(event: LogEvent) -> Self { 
        Self { time: Local::now(), event }
    }
}
impl std::fmt::Display for LogEntry { 
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}.{:02}| {:?}", 
            self.time.hour(),
            self.time.minute(),
            self.time.second(),
            self.event
        )
    }
}


#[derive(Debug)]
pub enum LogEvent {
    Acquire,
    Exposure(usize),
    AnalogGain(usize),
    Msg(usize),
    CameraMsg(CameraMessage),
    LostThread,
}


