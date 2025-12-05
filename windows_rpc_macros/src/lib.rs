mod client_codegen;
#[allow(dead_code)]
mod constants;
mod ndr;
mod ndr64;
mod parse;
mod server_codegen;
mod types;

use quote::ToTokens;
use syn::{FnArg, ReturnType, TraitItem};
use windows::core::GUID;

use client_codegen::compile_client;
use parse::InterfaceAttributes;
use server_codegen::compile_server;
use types::{Interface, Method, Parameter, Type};

// FIXME: simplify by extracting to method that return Result<proc_macro2::TokenStream, Error>
#[proc_macro_attribute]
pub fn rpc_interface(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Parse interface attributes (guid and version)
    let attrs: InterfaceAttributes = match syn::parse(attr) {
        Ok(attrs) => attrs,
        Err(e) => return e.to_compile_error().into(),
    };

    let input_clone = input.clone();
    let t = match syn::parse2(input.into()) {
        Err(e) => {
            return e.to_compile_error().into();
        }
        Ok(syn::Item::Trait(t)) => t,
        Ok(unrecognized) => {
            return syn::Error::new_spanned(
                &unrecognized,
                "The #[rpc_interface] attribute cannot be used with this kind of item.",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut methods = vec![];
    for item in t.items {
        let TraitItem::Fn(func) = item else {
            return syn::Error::new_spanned(
                proc_macro2::TokenStream::from(input_clone),
                "Only functions are allowed on this trait",
            )
            .into_compile_error()
            .into();
        };

        let return_type = match func.sig.output {
            ReturnType::Default => None,
            ReturnType::Type(_, t) => match Type::try_from(*t) {
                Ok(t) => Some(t),
                Err(e) => return e.into_compile_error().into(),
            },
        };

        let mut params = vec![];
        for param in func.sig.inputs {
            let FnArg::Typed(typed) = param else {
                // FIXME: I'd like to support that (and even make that mandatory)
                return syn::Error::new_spanned(
                    proc_macro2::TokenStream::from(input_clone),
                    "Passing self is currently not supported",
                )
                .into_compile_error()
                .into();
            };

            let syn::Pat::Ident(param_name) = *typed.pat else {
                return syn::Error::new_spanned(typed.pat.to_token_stream(), "Expected identifier")
                    .into_compile_error()
                    .into();
            };

            let param_type = match Type::try_from(*typed.ty) {
                Ok(t) => t,
                Err(e) => return e.into_compile_error().into(),
            };

            params.push(Parameter {
                r#type: param_type,
                name: param_name.ident.to_string(),
                // FIXME: let mut affect this (can be in/out)
                is_in: true,
                is_out: false,
            });
        }

        methods.push(Method {
            return_type,
            name: func.sig.ident.to_string(),
            parameters: params,
        });
    }

    let interface = Interface {
        name: t.ident.to_string(),
        uuid: GUID::from_u128(attrs.guid),
        version: attrs.version,
        methods,
    };

    let client_code = compile_client(interface.clone());
    let server_code = compile_server(interface);

    quote::quote! {
        #client_code
        #server_code
    }
    .into()
}
