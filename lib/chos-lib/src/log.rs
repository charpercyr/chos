use core::fmt::{Arguments, Write};

use cfg_if::cfg_if;

use crate::sync::{RawLock, Lock};

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Critical,
    Always,
}

pub trait LogHandler {
    fn log(&self, args: Arguments<'_>, lvl: LogLevel);
    unsafe fn log_unsafe(&self, args: Arguments<'_>, lvl: LogLevel);
}

impl<L: RawLock, W: Write> LogHandler for Lock<L, W> {
    fn log(&self, args: Arguments<'_>, _: LogLevel) {
        let mut w = self.lock_noirq();
        drop(w.write_fmt(args))
    }
    unsafe fn log_unsafe(&self, args: Arguments<'_>, _: LogLevel) {
        let w = &mut *self.get_ptr();
        drop(w.write_fmt(args))
    }
}

pub struct TermColorLogHandler<H> {
    h: H,
}
impl<H> TermColorLogHandler<H> {
    pub const fn new(h: H) -> Self {
        Self { h }
    }
    fn apply_fmt<F>(&self, f: F, args: Arguments<'_>, lvl: LogLevel)
    where
        F: Fn(&H, Arguments<'_>, LogLevel),
    {
        let fmt = match lvl {
            LogLevel::Debug => "2",
            LogLevel::Info => "",
            LogLevel::Warn => "1;33",
            LogLevel::Error => "1;31",
            LogLevel::Critical => "1;37;41",
            LogLevel::Always => "",
        };
        f(
            &self.h,
            format_args!("\x1b[{fmt}m{args}\x1b[0m", fmt = fmt, args = args),
            lvl,
        );
    }
}
impl<H: LogHandler> LogHandler for TermColorLogHandler<H> {
    fn log(&self, args: Arguments<'_>, lvl: LogLevel) {
        self.apply_fmt(H::log, args, lvl)
    }
    unsafe fn log_unsafe(&self, args: Arguments<'_>, lvl: LogLevel) {
        self.apply_fmt(|this, args, lvl| H::log_unsafe(this, args, lvl), args, lvl)
    }
}

static mut LOG_HANDLER: Option<&'static dyn LogHandler> = None;

pub unsafe fn set_handler(handler: &'static dyn LogHandler) {
    LOG_HANDLER = Some(handler);
}

pub unsafe fn clear_handler() {
    LOG_HANDLER = None;
}

pub fn log_impl(args: Arguments<'_>, lvl: LogLevel) {
    if let Some(handler) = unsafe { LOG_HANDLER } {
        handler.log(args, lvl);
    }
}

pub unsafe fn unsafe_log_impl(args: Arguments<'_>, lvl: LogLevel) {
    if let Some(handler) = LOG_HANDLER {
        handler.log(args, lvl);
    }
}

pub macro print ($($args:tt)*) {
    $crate::log::log_impl(format_args!($($args)*), $crate::log::LogLevel::Info)
}

pub macro unsafe_print ($($args:tt)*) {
    $crate::log::unsafe_log_impl(format_args!($($args)*), $crate::log::LogLevel::Info)
}

pub macro log_impl {
    ($lvl:expr, $fmt:expr, $($args:tt)*) => {
        $crate::log::log_impl(format_args!(concat!($fmt, "\n"), $($args)*), $lvl)
    },
    ($lvl:expr, $fmt:expr $(,)?) => {
        $crate::log::log_impl(format_args!(concat!($fmt, "\n")), $lvl)
    },
    ($lvl:expr $(,)?) => {
        $crate::log::log_impl(format_args!("\n"), $lvl)
    },
}

pub macro log_unsafe_impl {
    ($lvl:expr, $fmt:expr, $($args:tt)*) => {
        $crate::log::unsafe_log_impl(format_args!(concat!($fmt, "\n"), $($args)*), $lvl)
    },
    ($lvl:expr, $fmt:expr $(,)?) => {
        $crate::log::unsafe_log_impl(format_args!(concat!($fmt, "\n")), $lvl)
    },
    ($lvl:expr $(,)?) => {
        $crate::log::unsafe_log_impl(format_args!("\n"), $lvl)
    },
}

pub macro println ($($args:tt)*) {
    $crate::log::log_impl!($crate::log::LogLevel::Always, $($args)*)
}

pub macro unsafe_println ($($args:tt)*) {
    $crate::log::log_unsafe_impl!($crate::log::LogLevel::Always, $($args)*)
}

cfg_if! {
    if #[cfg(feature = "log-debug")] {
        pub macro debug ($($args:tt)*) {
            $crate::log::log_impl!($crate::log::LogLevel::Debug, $($args)*)
        }
        pub macro unsafe_debug ($($args:tt)*) {
            $crate::log::log_unsafe_impl!($crate::log::LogLevel::Debug, $($args)*)
        }
    } else {
        pub macro debug ($($args:tt)*) {{}}
        pub macro unsafe_debug ($($args:tt)*) {{}}
    }
}

cfg_if! {
    if #[cfg(feature = "log-info")] {
        pub macro info ($($args:tt)*) {
            $crate::log::log_impl!($crate::log::LogLevel::Info, $($args)*)
        }
        pub macro unsafe_info ($($args:tt)*) {
            $crate::log::log_unsafe_impl!($crate::log::LogLevel::Info, $($args)*)
        }
    } else {
        pub macro info ($($args:tt)*) {{}}
        pub macro unsafe_info ($($args:tt)*) {{}}
    }
}

cfg_if! {
    if #[cfg(feature = "log-warn")] {
        pub macro warn ($($args:tt)*) {
            $crate::log::log_impl!($crate::log::LogLevel::Warn, $($args)*)
        }
        pub macro unsafe_warn ($($args:tt)*) {
            $crate::log::log_unsafe_impl!($crate::log::LogLevel::Warn, $($args)*)
        }
    } else {
        pub macro warn ($($args:tt)*) {{}}
        pub macro unsafe_warn ($($args:tt)*) {{}}
    }
}

cfg_if! {
    if #[cfg(feature = "log-error")] {
        pub macro error ($($args:tt)*) {
            $crate::log::log_impl!($crate::log::LogLevel::Error, $($args)*)
        }
        pub macro unsafe_error ($($args:tt)*) {
            $crate::log::log_unsafe_impl!($crate::log::LogLevel::Error, $($args)*)
        }
    } else {
        pub macro error ($($args:tt)*) {{}}
        pub macro unsafe_error ($($args:tt)*) {{}}
    }
}

cfg_if! {
    if #[cfg(feature = "log-critical")] {
        pub macro critical ($($args:tt)*) {
            $crate::log::log_impl!($crate::log::LogLevel::Critical, $($args)*)
        }
        pub macro unsafe_critical ($($args:tt)*) {
            $crate::log::log_unsafe_impl!($crate::log::LogLevel::Critical, $($args)*)
        }
    } else {
        pub macro critical ($($args:tt)*) {{}}
        pub macro unsafe_critical ($($args:tt)*) {{}}
    }
}

pub trait Domain {
    fn name(&self) -> &str;
    fn enabled(&self) -> bool;
}

pub macro domain_println($domain:expr, $($args:tt)*) {
    if $crate::log::Domain::enabled(&$domain) {
        $crate::log::println!("{}: {}", $domain.name(), format_args!($($args)*));
    }
}

pub macro unsafe_domain_println($domain:expr, $($args:tt)*) {
    if $crate::log::Domain::enabled(&$domain) {
        $crate::log::unsafe_println!("{}: {}", $domain.name(), format_args!($($args)*));
    }
}

pub macro domain_debug ($domain:expr, $($args:tt)*) {
    if $crate::log::Domain::enabled(&$domain) {
        $crate::log::debug!("{}: {}", $domain.name(), format_args!($($args)*));
    }
}

pub macro unsafe_domain_debug ($domain:expr, $($args:tt)*) {
    if $crate::domain::Domain::enabled(&$domain) {
        $crate::log::unsafe_debug!("{}: {}", $domain.name(), format_args!($($args)*));
    }
}

pub macro domain_info ($domain:expr, $($args:tt)*) {
    if $crate::domain::Domain::enabled(&$domain) {
        $crate::log::info!($($args)*);
    }
}

pub macro unsafe_domain_info ($domain:expr, $($args:tt)*) {
    if $crate::log::Domain::enabled(&$domain) {
        $crate::log::unsafe_info!($($args)*);
    }
}

pub macro domain_warn ($domain:expr, $($args:tt)*) {
    if $crate::domain::Domain::enabled(&$domain) {
        $crate::log::warn!($($args)*);
    }
}

pub macro unsafe_domain_warn ($domain:expr, $($args:tt)*) {
    if $crate::log::Domain::enabled(&$domain) {
        $crate::log::unsafe_warn!($($args)*);
    }
}

pub macro domain_error ($domain:expr, $($args:tt)*) {
    if $crate::log::Domain::enabled($domain) {
        $crate::log::error!($($args)*);
    }
}

pub macro unsafe_domain_error ($domain:expr, $($args:tt)*) {
    if $crate::log::Domain::enabled(&$domain) {
        $crate::log::unsafe_error!($($args)*);
    }
}

pub macro domain_critical ($domain:expr, $($args:tt)*) {
    if $crate::log::Domain::enabled(&$domain) {
        $crate::log::critical!($($args)*);
    }
}

pub macro unsafe_domain_critical ($domain:expr, $($args:tt)*) {
    if $crate::log::Domain::enabled(&$domain) {
        $crate::log::unsafe_critical!($($args)*);
    }
}

pub macro domain ($($name:ident = $value:expr),* $(,)?) {
    $(
        paste::item! {
            pub struct [<__ $name:camel>];
            pub const $name: [<__ $name:camel>] = [<__ $name:camel>];
            impl $crate::log::Domain for [<__ $name:camel>] {
                #[inline]
                fn name(&self) -> &str {
                    stringify!($name)
                }
                #[inline]
                fn enabled(&self) -> bool {
                    $value
                }
            }
        }
    )*
}

pub macro todo_warn($($args:tt)*) {
    $crate::log::warn!(concat!("TODO ", file!(), ":", line!(), " {}"), format_args!($($args)*))
}

pub macro unsafe_todo_warn($($args:tt)*) {
    $crate::log::unsafe_warn!(concat!(file!(), ":", line!(), " TODO {}"), format_args!($($args)*))
}
