
use cfg_if::cfg_if;

macro_rules! use_arch {
    ($mod:ident, $arch:expr) => {
        cfg_if! {
            if #[cfg(target_arch = $arch)] {
                #[macro_use]
                mod $mod;
                pub use self::$mod::*;
            }
        }
    };
}

use_arch!(x86_64, "x86_64");
