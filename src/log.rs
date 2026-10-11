use std::sync::atomic::{AtomicU32, Ordering};

#[allow(dead_code)]
#[repr(u32)]
pub enum Level {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

static CURRENT_LEVEL: AtomicU32 = AtomicU32::new(Level::Warn as u32);

pub fn set_log_level(level: Level) {
    CURRENT_LEVEL.store(level as u32, Ordering::Release);
}

pub(crate) fn debug_print_(level: u32, args: std::fmt::Arguments) {
    let current_level = CURRENT_LEVEL.fetch_add(0, Ordering::Acquire);
    if level <= current_level {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let level = match level {
                1 => "ERROR",
                2 => "WARN",
                3 => "INFO",
                4 => "DEBUG",
                _ => "TRACE",
            };
            println!("{}: {}", level, args);
        }
        #[cfg(target_arch = "wasm32")]
        {
            use crate::peer;

            if let Some(s) = args.as_str() {
                let s = s.as_bytes();
                unsafe {
                    peer::debug_print(level, s.as_ptr(), s.len());
                }
            } else {
                let s = args.to_string();
                let s = s.as_bytes();
                unsafe {
                    peer::debug_print(level, s.as_ptr(), s.len());
                }
            }
        }
    }
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {{
        $crate::log::debug_print_($crate::log::Level::Error as u32, std::format_args!($($arg)*));
    }};
    () => { };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {{
        $crate::log::debug_print_($crate::log::Level::Warn as u32, std::format_args!($($arg)*));
    }};
    () => { };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        $crate::log::debug_print_($crate::log::Level::Info as u32, std::format_args!($($arg)*));
    }};
    () => { };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {{
        $crate::log::debug_print_($crate::log::Level::Debug as u32, std::format_args!($($arg)*));
    }};
    () => { };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {{
        $crate::log::debug_print_($crate::log::Level::Trace as u32, std::format_args!($($arg)*));
    }};
    () => { };
}
