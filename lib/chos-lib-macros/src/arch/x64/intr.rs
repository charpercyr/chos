use proc_macro::TokenStream;
use quote::quote;

pub fn error(msg: &str) -> TokenStream {
    return quote! {
        compile_error!(#msg);
    }
    .into();
}

fn signature_to_bare_fn(sig: &syn::Signature) -> Result<syn::TypeBareFn, TokenStream> {
    Ok(syn::TypeBareFn {
        abi: sig.abi.clone(),
        fn_token: sig.fn_token.clone(),
        inputs: sig.inputs.iter().map(|arg| match arg {
            syn::FnArg::Typed(typ) => Ok({
                syn::BareFnArg {
                    attrs: typ.attrs.clone(),
                    name: None,
                    ty: (*typ.ty).clone(),
                }
            }),
            _ => Err(error("Cannot take self as argument")),
        }).try_collect()?,
        lifetimes: None,
        output: sig.output.clone(),
        paren_token: sig.paren_token.clone(),
        unsafety: sig.unsafety.clone(),
        variadic: sig.variadic.clone(),
    })
}

pub fn parse_interrupt(attr: TokenStream, items: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return error("Attributes must be empty");
    }
    let syn::ItemFn {
        attrs,
        block,
        sig,
        vis,
    } = syn::parse_macro_input!(items as syn::ItemFn);
    if sig
        .abi
        .as_ref()
        .and_then(|abi| abi.name.as_ref())
        .as_ref()
        .map(|&name| name.value())
        .as_deref()
        != Some("x86-interrupt")
    {
        return error("Invalid ABI, must use 'x86-interrupt'");
    }
    if !sig.generics.params.is_empty() {
        return error("Cannot have generics");
    }
    let name = sig.ident.clone();
    let bare_sig = match signature_to_bare_fn(&sig) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let args = sig.inputs.clone();
    let has_extra = match args.len() {
        1 => false,
        2 => true,
        _ => return error("Invalid number of arguments"),
    };
    let ret = sig.output.clone();
    let diverges = match &ret {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, typ) => match &**typ {
            syn::Type::Never(_) => true,
            syn::Type::Tuple(tuple) => {
                if tuple.elems.len() == 0 {
                    false
                } else {
                    return error("Invalid return type");
                }
            }
            _ => return error("Invalid return type"),
        },
    };
    let asm_prologue = if has_extra {
        proc_macro2::TokenStream::new()
    } else {
        quote! {
            "pushq $-1",
        }
    };
    let asm_epilogue = if diverges {
        proc_macro2::TokenStream::new()
    } else {
        quote! {
            "pop %rax",
            "pop %r11",
            "pop %r10",
            "pop %r9",
            "pop %r8",
            "pop %rcx",
            "pop %rdx",
            "pop %rsi",
            "pop %rdi",
            "add $8, %rsp",
            "iretq",
        }
    };
    quote! {
        #[allow(non_upper_case_globals)]
        #vis static #name: chos_lib::arch::x64::tables::idt::HandlerFn<#bare_sig> = unsafe {
            chos_lib::arch::x64::tables::idt::HandlerFn::new({
                #(#attrs)*
                #[naked]
                #sig {
                    fn intr_handler(#args) #ret {
                        #block
                    }
                    unsafe {
                        core::arch::asm! {
                            #asm_prologue
                            "push %rdi",
                            "push %rsi",
                            "push %rdx",
                            "push %rcx",
                            "push %r8",
                            "push %r9",
                            "push %r10",
                            "push %r11",
                            "push %rax",
                            "mov %rsp, %rdi",
                            "mov 72(%rsp), %rsi",
                            "call {handler}",
                            #asm_epilogue
                            handler = sym intr_handler,
                            options(noreturn, att_syntax),
                        }
                    }
                }
                #name
            })
        };
    }
    .into()
}
