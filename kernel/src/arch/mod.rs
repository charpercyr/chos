
use cfg_if::cfg_if;

macro_rules! match_arch {
    ($($arch:expr => $amod:ident),* $(,)?) => {
        $(
            cfg_if! {
                if #[cfg(target_arch = $arch)] {
                    mod $amod;
                    pub use self::$amod::*;
                }
            }
        )*
    };
}

match_arch!(
    "x86_64" => x86_64,
    "arm" => arm,
);
