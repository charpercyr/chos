use proc_macro::TokenStream;

mod bitfield;
mod fmt;

#[proc_macro]
pub fn forward_fmt(items: TokenStream) -> TokenStream {
    fmt::parse_forward_fmt(items)
}

#[proc_macro]
pub fn bitfield(items: TokenStream) -> TokenStream {
    bitfield::parse_bitfield(items)
}