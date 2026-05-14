use libseat_sys::{libseat_log_level, libseat_set_log_level};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Silent = 0,
    Error = 1,
    Info = 2,
    Debug = 3,
    Last = 4,
}

impl From<libseat_log_level> for LogLevel {
    fn from(v: libseat_log_level) -> LogLevel {
        match v {
            0 => LogLevel::Silent,
            1 => LogLevel::Error,
            2 => LogLevel::Info,
            3 => LogLevel::Debug,
            _ => LogLevel::Last,
        }
    }
}

pub fn set_log_level(level: LogLevel) {
    unsafe { libseat_set_log_level(level as _) }
}
