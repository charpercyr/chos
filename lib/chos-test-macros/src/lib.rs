
use proc_macro::TokenStream;

use proc_macro2::Span;

use quote::quote;

#[proc_macro_attribute]
pub fn test(_: TokenStream, input: TokenStream) -> TokenStream {
    let fun = syn::parse_macro_input!(input as syn::ItemFn);
    let fun_ident = &fun.sig.ident;
    let fn_name = format!("{}", fun_ident);
    let static_name = syn::Ident::new(&format!("__CHOS_TEST_{}", fn_name), Span::call_site());
    let fun_name_str = format!("{}", fn_name);

    let res = quote! {
        #[used]
        #[link_section = ".testcases"]
        static #static_name: chos_test::TestCase = chos_test::TestCase {
            name: #fun_name_str,
            fun: #fun_ident,
        };
        #fun
    };
    res.into()
}
