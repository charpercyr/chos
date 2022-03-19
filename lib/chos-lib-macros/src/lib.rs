#![feature(iterator_try_collect)]

use proc_macro::TokenStream;

mod arch;
mod fmt;

#[proc_macro]
pub fn forward_fmt(items: TokenStream) -> TokenStream {
    fmt::parse_forward_fmt(items)
}

#[cfg(target_arch = "x86_64")]
#[proc_macro_attribute]
pub fn interrupt(attr: TokenStream, items: TokenStream) -> TokenStream {
    arch::parse_interrupt(attr, items)
}
