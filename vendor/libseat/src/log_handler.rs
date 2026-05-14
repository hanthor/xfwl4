use log::{debug, error, info, trace};
use std::{ffi::CStr, os::raw::c_char, sync::Once};

use crate::LogLevel;

type LogHandlerFn = unsafe extern "C" fn(level: LogLevel, msg: *const c_char);

extern "C" {
    fn init_preformatted_log_handler(handler: LogHandlerFn);
}

fn log(level: LogLevel, msg: &str) {
    match level {
        LogLevel::Silent => trace!("{}", msg),
        LogLevel::Error => error!("{}", msg),
        LogLevel::Info => info!("{}", msg),
        LogLevel::Debug | LogLevel::Last => debug!("{}", msg),
    }
}

extern "C" fn ffi_handler(level: LogLevel, msg: *const c_char) {
    let cstr = unsafe { CStr::from_ptr(msg) };
    match cstr.to_str() {
        Ok(msg) => log(level, msg),
        Err(err) => error!("{:?}", err),
    }
}

static INIT: Once = Once::new();

/// Initialise libseat log handler
pub fn init() {
    INIT.call_once(|| {
        crate::set_log_level(LogLevel::Debug);

        unsafe { init_preformatted_log_handler(ffi_handler) };
    })
}
