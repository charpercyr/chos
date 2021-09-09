
use core::fmt::Arguments;

use cfg_if::cfg_if;

pub type LogHandler = unsafe fn(Arguments<'_>, bool);

static mut LOG_HANDLER: Option<LogHandler> = None;

pub unsafe fn set_handler(handler: LogHandler) {
    LOG_HANDLER = Some(handler);
}

pub unsafe fn clear_handler() {
    LOG_HANDLER = None;
}

pub unsafe fn log_impl(args: Arguments<'_>, is_unsafe: bool) {
    if let Some(log) = LOG_HANDLER {
        log(args, is_unsafe);
    }
}

pub macro __log {
    ($uns:expr, $term_fmt:expr, $fmt:expr, $($args:tt)*) => {
        chos_lib::log::log_impl(format_args!(concat!("\x1b[", $term_fmt, "m", $fmt, "\x1b[0m"), $($args)*), $uns)
    },
    ($uns:expr, $term_fmt:expr, $fmt:expr) => {
        chos_lib::log::__log!($uns, $term_fmt, $fmt, )
    },
    ($uns:expr, $term_fmt:expr, ) => {
        chos_lib::log::__log!($uns, $term_fmt, "", )
    },
}

cfg_if! {
    if #[cfg(feature = "log-debug")] {
        pub macro debug ($($args:tt)*) {
            unsafe { chos_lib::log::__log!(false, "2", $($args)*) }
        }
        pub macro unsafe_debug ($($args:tt)*) {
            chos_lib::log::__log!(true, "2", $($args)*)
        }
    } else {
        pub macro debug ($($args:tt)*) {}
        pub macro unsafe_debug ($($args:tt)*) {}
    }
}

cfg_if!{
    if #[cfg(feature = "log-info")] {
        pub macro info ($($args:tt)*) {
            unsafe { chos_lib::log::__log!(false, "", $($args)*) }
        }
        pub macro unsafe_info($($args:tt)*) {
            chos_lib::log::__log!(true, "", $($args)*)
        }
    } else {
        pub macro info ($($args:tt)*) {}
        pub macro unsafe_info ($($args:tt)*) {}
    }
}

cfg_if!{
    if #[cfg(feature = "log-warn")] {
        pub macro warn ($($args:tt)*) {
            unsafe { chos_lib::log::__log!(false, "1;33", $($args)*) }
        }
        pub macro unsafe_warn($($args:tt)*) {
            chos_lib::log::__log!(true, "1;33", $($args)*)
        }
    } else {
        pub macro warn ($($args:tt)*) {}
        pub macro unsafe_warn ($($args:tt)*) {}
    }
}

cfg_if!{
    if #[cfg(feature = "log-error")] {
        pub macro error ($($args:tt)*) {
            unsafe { chos_lib::log::__log!(false, "1;31", $($args)*) }
        }
        pub macro unsafe_error($($args:tt)*) {
            chos_lib::log::__log!(true, "1;31", $($args)*)
        }
    } else {
        pub macro error ($($args:tt)*) {}
        pub macro unsafe_error ($($args:tt)*) {}
    }
}

cfg_if!{
    if #[cfg(feature = "log-critical")] {
        pub macro critical ($($args:tt)*) {
            unsafe { chos_lib::log::__log!(false, "1;37;41", $($args)*) }
        }
        pub macro unsafe_critical($($args:tt)*) {
            chos_lib::log::__log!(true, "1;37;41", $($args)*)
        }
    } else {
        pub macro critical ($($args:tt)*) {}
        pub macro unsafe_critical ($($args:tt)*) {}
    }
}
