
use core::fmt::Arguments;

use chos_lib::spin::Lock;

pub static LOG: Lock<Option<fn(Arguments)>> = Lock::new(None);
pub fn use_early_debug(f: fn(Arguments)) {
    let mut log = LOG.lock();
    *log = Some(f);
}

#[macro_export]
macro_rules! debug {
    ($($args:tt)*) => {{
        let log = $crate::log::LOG.lock();
        if let Some(log) = &*log {
            log(format_args!($($args)*));
        }
        drop(log);
    }};
}
