use proc_macro::TokenStream;

mod fmt;

#[proc_macro]
pub fn forward_fmt(items: TokenStream) -> TokenStream {
    fmt::parse_forward_fmt(items)
}
