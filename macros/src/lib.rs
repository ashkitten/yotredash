#![recursion_limit = "128"]
#![feature(box_patterns)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_attribute]
pub fn wrap_result(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = syn::parse_macro_input!(attr as syn::AttributeArgs);
    let err_call: Option<syn::Expr> = match attr_args.get(0) {
        Some(syn::NestedMeta::Literal(syn::Lit::Str(lit))) => Some(lit.parse().unwrap()),
        None => None,
        _ => unreachable!(),
    };

    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let name = &input.ident;
    let constness = &input.constness;
    let unsafety = &input.unsafety;
    let asyncness = &input.asyncness;
    let abi = &input.abi;
    let inputs = &input.decl.inputs;
    let inner_inputs: syn::punctuated::Punctuated<&dyn quote::ToTokens, syn::token::Comma> = input
        .decl
        .inputs
        .iter()
        .filter_map(|arg| -> Option<&dyn quote::ToTokens> {
            match arg {
                syn::FnArg::SelfRef(syn::ArgSelfRef { self_token, .. }) => Some(self_token),
                syn::FnArg::SelfValue(syn::ArgSelf { self_token, .. }) => Some(self_token),
                syn::FnArg::Captured(syn::ArgCaptured { pat, .. }) => Some(pat),
                syn::FnArg::Inferred(pat) => Some(pat),
                syn::FnArg::Ignored(_) => None,
            }
        })
        .collect();
    let output = match &input.decl.output {
        &syn::ReturnType::Type(_, box syn::Type::Path(ref path)) => {
            let mut segments = path.path.segments.clone();
            let last_segment = segments.pop().unwrap();
            match last_segment.into_value().arguments {
                syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    args,
                    ..
                }) => {
                    let out_type = args.into_iter().next().unwrap();
                    quote!(-> #out_type)
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    };
    let inner_output = &input.decl.output;
    let body = &input.block;

    let tokens = quote! {
        #constness #unsafety #asyncness #abi fn #name(#inputs) #output {
            use std::io::Write;
            use failure::Error;
            use log::{error, Level, log_enabled};

            fn inner(#inputs) #inner_output #body

            if let Err(error) = inner(#inner_inputs) {
                if log_enabled!(Level::Debug) {
                    let mut causes = error.iter_chain();

                    error!(
                        "{}",
                        causes
                        .next()
                        .expect("`causes` should contain at least one error")
                    );
                    for cause in causes {
                        error!("Caused by: {}", cause);
                    }

                    let backtrace = format!("{}", error.backtrace());
                    if backtrace.is_empty() {
                        writeln!(
                            ::std::io::stderr(),
                            "Set RUST_BACKTRACE=1 to see a backtrace"
                        )
                            .expect("Could not write to stderr");
                    } else {
                        writeln!(::std::io::stderr(), "{}", error.backtrace())
                            .expect("Could not write to stderr");
                    }
                }

                #err_call;
            };
        }
    };

    tokens.into()
}
