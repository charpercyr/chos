
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

pub macro print ($($args:tt)*) {
    #[allow(unused_unsafe)]
    unsafe { $crate::log::log_impl(format_args!($($args)*), false) }
}

pub macro println {
    ($fmt:expr, $($args:tt)*) => {
        #[allow(unused_unsafe)]
        unsafe { $crate::log::log_impl(format_args!(concat!($fmt, "\n"), $($args)*), false) }
    },
    ($fmt:expr $(,)?) => {
        #[allow(unused_unsafe)]
        unsafe { $crate::log::log_impl(format_args!(concat!($fmt, "\n")), false) }
    },
    () => {
        #[allow(unused_unsafe)]
        unsafe { $crate::log::log_impl(format_args!("\n"), false) }
    },
}

pub macro print_unsafe ($($args:tt)*) {
    $crate::log::log_impl(format_args!($($args)*), true)
}

pub macro println_unsafe {
    ($fmt:expr, $($args:tt)*) => {
        $crate::log::log_impl(format_args!(concat!($fmt, "\n"), $($args)*), true)
    },
    ($fmt:expr $(,)?) => {
        $crate::log::log_impl(format_args!(concat!($fmt, "\n")), true)
    },
    () => {
        $crate::log::log_impl(format_args!("\n"), true)
    },
}

pub macro log_term_fmt {
    ($uns:expr, $term_fmt:expr, $fmt:expr, $($args:tt)*) => {
        $crate::log::log_impl(
            format_args!(concat!("\x1b[", $term_fmt, "m", $fmt, "\x1b[0m\n"), $($args)*),
            $uns,
        )
    },
    ($uns:expr, $term_fmt:expr, $fmt:expr) => {
        $crate::log::log_term_fmt!($uns, $term_fmt, $fmt, )
    },
    ($uns:expr, $term_fmt:expr, ) => {
        $crate::log::log_term_fmt!($uns, $term_fmt, "", )
    },
}

cfg_if! {
    if #[cfg(feature = "log-debug")] {
        pub macro debug ($($args:tt)*) {
            #[allow(unused_unsafe)]
            unsafe { $crate::log::log_term_fmt!(false, "2", $($args)*) }
        }
        pub macro unsafe_debug ($($args:tt)*) {
            $crate::log::log_term_fmt!(true, "2", $($args)*)
        }
    } else {
        pub macro debug ($($args:tt)*) {}
        pub macro unsafe_debug ($($args:tt)*) {}
    }
}

cfg_if!{
    if #[cfg(feature = "log-info")] {
        pub macro info ($($args:tt)*) {
            #[allow(unused_unsafe)]
            unsafe { $crate::log::log_term_fmt!(false, "", $($args)*) }
        }
        pub macro unsafe_info($($args:tt)*) {
            $crate::log::log_term_fmt!(true, "", $($args)*)
        }
    } else {
        pub macro info ($($args:tt)*) {}
        pub macro unsafe_info ($($args:tt)*) {}
    }
}

cfg_if!{
    if #[cfg(feature = "log-warn")] {
        pub macro warn ($($args:tt)*) {
            #[allow(unused_unsafe)]
            unsafe { $crate::log::log_term_fmt!(false, "1;33", $($args)*) }
        }
        pub macro unsafe_warn($($args:tt)*) {
            $crate::log::log_term_fmt!(true, "1;33", $($args)*)
        }
    } else {
        pub macro warn ($($args:tt)*) {}
        pub macro unsafe_warn ($($args:tt)*) {}
    }
}

cfg_if!{
    if #[cfg(feature = "log-error")] {
        pub macro error ($($args:tt)*) {
            #[allow(unused_unsafe)]
            unsafe { $crate::log::log_term_fmt!(false, "1;31", $($args)*) }
        }
        pub macro unsafe_error($($args:tt)*) {
            $crate::log::log_term_fmt!(true, "1;31", $($args)*)
        }
    } else {
        pub macro error ($($args:tt)*) {}
        pub macro unsafe_error ($($args:tt)*) {}
    }
}

cfg_if!{
    if #[cfg(feature = "log-critical")] {
        pub macro critical ($($args:tt)*) {
            #[allow(unused_unsafe)]
            unsafe { $crate::log::log_term_fmt!(false, "1;37;41", $($args)*) }
        }
        pub macro unsafe_critical($($args:tt)*) {
            $crate::log::log_term_fmt!(true, "1;37;41", $($args)*)
        }
    } else {
        pub macro critical ($($args:tt)*) {}
        pub macro unsafe_critical ($($args:tt)*) {}
    }
}
