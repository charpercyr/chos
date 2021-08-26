use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Generics, Ident, Token, Type, WhereClause};

fn all_fmts(span: Span) -> HashSet<Ident> {
    [
        "Debug", "Display", "Pointer", "Binary", "Octal", "LowerHex", "UpperHex", "LowerExp",
        "UpperExp",
    ]
    .iter()
    .map(|&d| Ident::new(d, span.clone()))
    .collect()
}

struct ForwardFmt {
    generics: Option<Generics>,
    fmts: HashSet<Ident>,
    wh: Option<WhereClause>,
    ty: Type,
    field: Expr,
    fty: Type,
}

impl Parse for ForwardFmt {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![impl]>()?;
        let generics = input.parse::<Generics>().ok();
        let mut fmts = None;
        loop {
            let fmt: Ident = input.parse()?;
            if fmt == "ALL" {
                fmts = Some(all_fmts(fmt.span()))
            } else {
                if fmts.is_none() {
                    fmts = Some(HashSet::new())
                }
                fmts.as_mut().unwrap().insert(fmt);
            }
            if input.parse::<Token![,]>().is_err() {
                break;
            }
        }
        if fmts.is_none() {
            input.error("Need at least one format trait");
        }
        input.parse::<Token![for]>()?;
        let ty: Type = input.parse()?;
        let wh = input.parse::<WhereClause>().ok();
        input.parse::<Token![=>]>()?;
        let fty: Type = input.parse()?;
        input.parse::<Token![:]>()?;
        let field: Expr = input.parse()?;
        Ok(ForwardFmt {
            generics,
            fmts: fmts.unwrap(),
            ty,
            field,
            wh,
            fty,
        })
    }
}

/**
 * impl<T: Copy, P: ReadAccess>
 *
 */
pub fn parse_forward_fmt(items: TokenStream) -> TokenStream {
    let ForwardFmt {
        generics,
        fmts,
        ty,
        wh,
        field,
        fty,
    } = syn::parse_macro_input!(items as ForwardFmt);
    let wh = wh.map(|w| w.predicates);
    let impls: Vec<_> = fmts
        .into_iter()
        .map(|f| {
            quote! {
                impl #generics core::fmt::#f for #ty where #fty: core::fmt::#f, #wh {
                    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                        let field = (#field)(self);
                        core::fmt::#f::fmt(&field, f)
                    }
                }
            }
        })
        .collect();
    let expanded = quote! {
        #(#impls)*
    };
    expanded.into()
}
