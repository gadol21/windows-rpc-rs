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
    match rpc_interface_inner(attr.into(), input.into()) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

fn rpc_interface_inner(
    attr: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    // Parse interface attributes (guid and version)
    let attrs: InterfaceAttributes = syn::parse2(attr)?;

    let input_clone = input.clone();
    let t: syn::ItemTrait = syn::parse2(input.into())?;

    let mut methods = vec![];
    for item in t.items {
        let TraitItem::Fn(func) = item else {
            return Err(syn::Error::new_spanned(
                proc_macro2::TokenStream::from(input_clone),
                "Only functions are allowed on this trait",
            ));
        };

        let return_type = match func.sig.output {
            ReturnType::Default => None,
            ReturnType::Type(_, t) => Some(Type::try_from(*t)?),
        };

        let mut params = vec![];
        for param in func.sig.inputs {
            let FnArg::Typed(typed) = param else {
                return Err(syn::Error::new_spanned(
                    proc_macro2::TokenStream::from(input_clone),
                    "Passing self is currently not supported",
                ));
            };

            let syn::Pat::Ident(param_name) = *typed.pat else {
                return Err(syn::Error::new_spanned(
                    typed.pat.to_token_stream(),
                    "Expected identifier",
                ));
            };

            let param_type = Type::try_from(*typed.ty)?;

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

    let client_code = compile_client(&interface);
    let server_code = compile_server(&interface);

    Ok(quote::quote! {
        #client_code
        #server_code
    })
}
