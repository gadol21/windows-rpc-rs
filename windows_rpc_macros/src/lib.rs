#[allow(dead_code)]
mod constants;
mod types;
mod parse;
mod ndr;
mod ndr64;
mod codegen;

use quote::ToTokens;
use syn::{FnArg, ReturnType, TraitItem};
use windows::core::GUID;

use codegen::compile_client;
use parse::InterfaceAttributes;
use types::{BaseType, Interface, InterfaceVersion, Method, Parameter, Type};

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
                "The #[trace_logging_provider] attribute cannot be used with this kind of item.",
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

    compile_client(interface).into()
}

#[proc_macro]
pub fn gen_interface(_item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    compile_client(Interface {
        name: "Hello".to_string(),
        uuid: GUID::from_u128(0x7a98c250_6808_11cf_b73b_00aa00b677a7),
        methods: vec![
            Method {
                return_type: Some(Type::Simple(BaseType::U64)),
                name: "NoParams".to_string(),
                parameters: vec![],
            },
            Method {
                return_type: Some(Type::Simple(BaseType::I32)),
                name: "SingleParamReturn".to_string(),
                parameters: vec![Parameter {
                    r#type: Type::Simple(BaseType::I32),
                    name: "foo".to_owned(),
                    is_in: true,
                    is_out: false,
                }],
            },
            Method {
                return_type: Some(Type::Simple(BaseType::I32)),
                name: "Sum".to_string(),
                parameters: vec![
                    Parameter {
                        r#type: Type::Simple(BaseType::I32),
                        name: "a".to_owned(),
                        is_in: true,
                        is_out: false,
                    },
                    Parameter {
                        r#type: Type::Simple(BaseType::I32),
                        name: "b".to_owned(),
                        is_in: true,
                        is_out: false,
                    },
                ],
            },
        ],
        version: InterfaceVersion::default(),
        ..Default::default()
    })
    .into()
}
