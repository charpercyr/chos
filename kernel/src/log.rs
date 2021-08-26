use core::fmt::Arguments;

use chos_lib::sync::lock::Spinlock;

pub static LOG: Spinlock<Option<fn(Arguments)>> = Spinlock::new(None);
pub fn use_early_debug(f: fn(Arguments)) {
    let mut log = LOG.lock();
    *log = Some(f);
}

#[macro_export]
macro_rules! log {
    ($term_fmt:expr, $fmt:expr, $($args:tt)*) => {
        let log = $crate::log::LOG.lock();
        if let Some(log) = &*log {
            log(format_args!(concat!("\x1b[", $term_fmt, "m", $fmt, "\x1b[0m"), $($args)*));
        }
        drop(log);
    };
    ($term_fmt:expr, $fmt:expr) => {
        log!($term_fmt, $fmt, )
    };
}

#[cfg(feature = "log-debug")]
#[macro_export]
macro_rules! debug {
    ($($args:tt)*) => {
        $crate::log!("2", $($args)*)
    };
}
#[cfg(not(feature = "log-debug"))]
#[macro_export]
macro_rules! debug {
    ($($args:tt)*) => {};
}

#[cfg(feature = "log-info")]
#[macro_export]
macro_rules! info {
    ($($args:tt)*) => {
        $crate::log!("", $($args)*)
    };
}
#[cfg(not(feature = "log-info"))]
#[macro_export]
macro_rules! info {
    ($($args:tt)*) => {};
}

#[cfg(feature = "log-warn")]
#[macro_export]
macro_rules! warn {
    ($($args:tt)*) => {
        $crate::log!("1;33", $($args)*)
    };
}
#[cfg(not(feature = "log-warn"))]
#[macro_export]
macro_rules! warn {
    ($($args:tt)*) => {};
}

#[cfg(feature = "log-error")]
#[macro_export]
macro_rules! error {
    ($($args:tt)*) => {
        $crate::log!("1;31", $($args)*)
    };
}
#[cfg(not(feature = "log-error"))]
#[macro_export]
macro_rules! error {
    ($($args:tt)*) => {};
}

#[cfg(feature = "log-critical")]
#[macro_export]
macro_rules! critical {
    ($($args:tt)*) => {
        $crate::log!("1;37;41", $($args)*)
    };
}
#[cfg(not(feature = "log-critical"))]
#[macro_export]
macro_rules! critical {
    ($($args:tt)*) => {};
}
