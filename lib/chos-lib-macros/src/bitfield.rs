use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenStream as TS2};

use syn::{
    bracketed,
    parse::{Parse, ParseStream},
};

use quote::quote;

#[derive(Debug)]
enum BitfieldOption {
    Debug,
    Eq,
    Visibility(syn::Visibility),
}

impl Parse for BitfieldOption {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        bracketed!(content in input);
        let typ = content.parse::<syn::Ident>()?;
        match &*typ.to_string() {
            "imp" => {
                let impl_type = content.parse::<syn::Ident>()?;
                match &*impl_type.to_string() {
                    "Debug" => Ok(Self::Debug),
                    "Eq" => Ok(Self::Eq),
                    _ => Err(content.error("Invalid impl")),
                }
            },
            "vis" => Ok(Self::Visibility(content.parse()?)),
            _ => Err(content.error("Invalid Option"))
        }
    }
}

#[derive(Debug)]
struct BitfieldAccessor {
    vis: Option<syn::Visibility>,
    name: syn::Ident,
}

impl Parse for BitfieldAccessor {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis = input.parse::<syn::Visibility>()?;
        let vis = if let syn::Visibility::Inherited = &vis {
            None
        } else {
            Some(vis)
        };
        let name = input.parse::<syn::Ident>()?;
        Ok(Self { vis, name })
    }
}

#[derive(Debug)]
struct BitfieldField {
    getter: BitfieldAccessor,
    setter: Option<BitfieldAccessor>,
    hig: usize,
    low: usize,
    typ: Option<syn::Type>,
}

impl Parse for BitfieldField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let getter = input.parse::<BitfieldAccessor>()?;
        let mut setter = None;
        if input.parse::<syn::Token!(,)>().is_ok() {
            setter = Some(input.parse::<BitfieldAccessor>()?);
        }
        input.parse::<syn::Token!(:)>()?;
        let hig = input.parse::<syn::LitInt>()?;
        let hig = hig.base10_parse()?;
        let low = if let Ok(_) = input.parse::<syn::Token!(,)>() {
            let low = input.parse::<syn::LitInt>()?;
            low.base10_parse()?
        } else {
            hig
        };
        if hig < low {
            return Err(input.error("High bit index should come first"));
        }
        let mut typ = None;
        if input.parse::<syn::Token!(->)>().is_ok() {
            typ = Some(input.parse::<syn::Type>()?);
        }
        input.parse::<syn::Token!(;)>()?;
        Ok(Self {
            getter,
            setter,
            hig,
            low,
            typ,
        })
    }
}

#[derive(Debug)]
struct Bitfield {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    name: syn::Ident,
    generics: Option<syn::Generics>,
    repr: syn::Type,
    fields: Vec<BitfieldField>,
    default_field_vis: Option<syn::Visibility>,
    debug: bool,
    eq: bool,
}

impl Parse for Bitfield {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer).unwrap_or_default();
        let vis = input
            .parse::<syn::Visibility>()
            .unwrap_or(syn::Visibility::Inherited);
        input.parse::<syn::Token!(struct)>()?;
        let name = input.parse::<syn::Ident>()?;

        let generics = input.parse::<syn::Generics>().ok();

        let repr;
        syn::parenthesized!(repr in input);
        let repr = repr.parse::<syn::Type>()?;

        let content;
        syn::braced!(content in input);

        let mut fields = Vec::new();
        let mut default_field_vis = None;
        let mut debug = false;
        let mut eq = false;
        while !content.is_empty() {
            if let Ok(opt) = content.parse::<BitfieldOption>() {
                match opt {
                    BitfieldOption::Debug => debug = true,
                    BitfieldOption::Eq => eq = true,
                    BitfieldOption::Visibility(vis) => default_field_vis = Some(vis),
                }
            } else if let Ok(field) = content.parse::<BitfieldField>() {
                fields.push(field);
            } else {
                return Err(input.error("Expected field or option"));
            }
        }
        Ok(Self {
            attrs,
            vis,
            name,
            generics,
            repr,
            fields,
            default_field_vis,
            debug,
            eq,
        })
        
    }
}

#[derive(Debug)]
struct BitfieldList {
    bfs: Vec<Bitfield>,
}

impl Parse for BitfieldList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut bfs = Vec::new();
        while !input.is_empty() {
            let bf = input.parse::<Bitfield>()?;
            bfs.push(bf);
        }
        Ok(Self {
            bfs,
        })
    }
}

fn impl_field(repr: &syn::Type, default_vis: Option<&syn::Visibility>, field: &BitfieldField) -> TS2 {
    let BitfieldField { getter, setter, hig, low, typ } = field;
    
    let typ = if typ.is_none() && hig == low {
        syn::Type::Path(syn::TypePath {
            qself: None,
            path: syn::Path {
                leading_colon: None,
                segments: vec![syn::PathSegment {
                    arguments: syn::PathArguments::None,
                    ident: syn::Ident::new("bool", proc_macro2::Span::call_site()),
                }].into_iter().collect(),
            }
        })
    } else {
        typ.as_ref().unwrap_or(repr).clone()
    };

    let hig = Literal::usize_suffixed(*hig);
    let low = Literal::usize_suffixed(*low);

    let BitfieldAccessor { vis: gvis, name: gname } = getter;
    let gvis = gvis.as_ref().or(default_vis).unwrap_or(&syn::Visibility::Inherited);
    let getter = quote! {
        #gvis fn #gname (&self) -> #typ {
            FieldRead::<#repr>::from_repr(Bitfield::get_bits(&self.bits, #low, #hig))
        }
    };
    let setter = setter.as_ref().map(|setter| {
        let BitfieldAccessor { vis: svis, name: sname } = setter;
        let svis = svis.as_ref().or(default_vis).unwrap_or(&syn::Visibility::Inherited);
        quote! {
            #svis fn #sname (&mut self, value: #typ) -> &mut Self {
                BitfieldMut::set_bits(&mut self.bits, #low, #hig, FieldWrite::<#repr>::into_repr(value));
                self
            }
        }
    });
    quote! {
        #getter
        #setter
    }
}

fn impl_fields(repr: &syn::Type, default_vis: Option<&syn::Visibility>, fields: &[BitfieldField]) -> TS2 {
    let field_impls: Vec<_> = fields.iter().map(|f| impl_field(repr, default_vis, f)).collect();
    quote! {
        #(#field_impls)*
    }
}

fn impl_fmt_debug(struct_name: &syn::Ident, fields: &[BitfieldField]) -> TS2 {
    let dbg_fields = fields.iter().map(|f| {
        let BitfieldField { getter, ..} = f;
        let BitfieldAccessor { name, ..} = getter;
        let name_str = name.to_string();
        let name_str = Literal::string(&name_str);
        quote! {
            .field(#name_str, &self.#name())
        }
    }).collect::<Vec<_>>();
    let struct_name_str = struct_name.to_string();
    let struct_name_str = Literal::string(&struct_name_str);
    quote! {
        impl core::fmt::Debug for #struct_name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct(#struct_name_str)
                #(#dbg_fields)*
                .finish()
            }
        }
    }
}

fn impl_eq(struct_name: &syn::Ident, fields: &[BitfieldField]) -> TS2 {
    let eq_fields = fields.iter().map(|f| {
        let BitfieldField{ getter, ..} = f;
        let BitfieldAccessor { name, ..} = getter;
        quote! {
            self.#name() == rhs.#name()
        }
    });
    quote! {
        impl core::cmp::PartialEq for #struct_name {
            fn eq(&self, rhs: &Self) -> bool {
                true #(&& #eq_fields)*
            }
        }

        impl core::cmp::Eq for #struct_name {}
    }
}

pub fn parse_bitfield(items: TokenStream) -> TokenStream {
    let bfs = syn::parse_macro_input!(items as BitfieldList);
    let mut res = Vec::new();
    for bf in bfs.bfs {
        let Bitfield {
            attrs,
            vis,
            name,
            generics,
            repr,
            fields,
            default_field_vis,
            debug,
            eq,
        } = bf;
        let field_stream = impl_fields(&repr, default_field_vis.as_ref(), &fields);
        let debug_stream = debug.then(|| impl_fmt_debug(&name, &fields));
        let partialeq_stream = eq.then(|| impl_eq(&name, &fields));
        res.push(quote! {
            #(#attrs)*
            #vis struct #name #generics {
                pub bits: #repr,
            }
    
            impl #name {
                pub const fn new(bits: #repr) -> Self {
                    Self { bits }
                }
    
                #field_stream
            }
    
            #debug_stream
            #partialeq_stream
        })
    }
    let res = quote! {
        #(#res)*
    };
    res.into()
}
