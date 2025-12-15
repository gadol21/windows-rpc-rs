//! Procedural macros for generating Windows RPC client and server code.
//!
//! This crate provides the [`macro@rpc_interface`] attribute macro that transforms
//! Rust trait definitions into fully functional Windows RPC clients and servers.
//!
//! See the [`windows_rpc`](https://docs.rs/windows-rpc) crate for the main documentation and examples.

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

use client_codegen::compile_client;
use parse::InterfaceAttributes;
use server_codegen::compile_server;
use types::{Interface, Method, Parameter, Type};

/// Generates Windows RPC client and server code from a trait definition.
///
/// This attribute macro transforms a Rust trait into a complete Windows RPC interface,
/// generating both client and server implementations that handle all the NDR marshalling,
/// format strings, and Windows RPC runtime integration automatically.
///
/// # Arguments
///
/// The macro requires two arguments:
///
/// - `guid(...)` - A unique interface identifier (UUID/GUID) in hexadecimal format
/// - `version(major.minor)` - The interface version number
///
/// # Generated Types
///
/// For a trait named `MyInterface`, the macro generates:
///
/// - **`MyInterfaceClient`** - A struct for making RPC calls to a server
/// - **`MyInterfaceServerImpl`** - A trait to implement for hosting a server
/// - **`MyInterfaceServer`** - A struct that wraps your implementation and handles RPC dispatch
///
/// # Supported Types
///
/// The following Rust types can be used for parameters and return values:
///
/// | Rust Type | NDR Type | Notes |
/// |-----------|----------|-------|
/// | `i8` | FC_SMALL | Signed 8-bit integer |
/// | `u8` | FC_USMALL | Unsigned 8-bit integer |
/// | `i16` | FC_SHORT | Signed 16-bit integer |
/// | `u16` | FC_USHORT | Unsigned 16-bit integer |
/// | `i32` | FC_LONG | Signed 32-bit integer |
/// | `u32` | FC_ULONG | Unsigned 32-bit integer |
/// | `i64` | FC_HYPER | Signed 64-bit integer |
/// | `u64` | FC_HYPER | Unsigned 64-bit integer |
/// | `&str` | Conformant string | Input parameters only |
/// | `String` | Conformant string | Return values only |
///
/// # Example
///
/// ```rust,ignore
/// use windows_rpc::rpc_interface;
/// use windows_rpc::client_binding::{ClientBinding, ProtocolSequence};
///
/// // Define the RPC interface
/// #[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
/// trait Calculator {
///     fn add(a: i32, b: i32) -> i32;
///     fn multiply(x: i32, y: i32) -> i32;
///     fn greet(name: &str) -> String;
/// }
///
/// // Implement the server
/// struct CalculatorImpl;
/// impl CalculatorServerImpl for CalculatorImpl {
///     fn add(&self, a: i32, b: i32) -> i32 {
///         a + b
///     }
///     fn multiply(&self, x: i32, y: i32) -> i32 {
///         x * y
///     }
///     fn greet(&self, name: &str) -> String {
///         format!("Hello, {name}!")
///     }
/// }
///
/// // Start the server
/// let mut server = CalculatorServer::new(CalculatorImpl);
/// server.register("my_endpoint").expect("Failed to register");
/// server.listen_async().expect("Failed to listen");
///
/// // Create a client and call methods
/// let binding = ClientBinding::new(ProtocolSequence::Alpc, "my_endpoint")
///     .expect("Failed to create binding");
/// let client = CalculatorClient::new(binding);
///
/// assert_eq!(client.add(10, 20), 30);
/// assert_eq!(client.multiply(5, 6), 30);
///
/// server.stop().expect("Failed to stop");
/// ```
///
/// # Limitations
///
/// - Only ALPC (local RPC) protocol is currently supported
/// - No support for input-output (`[in, out]`) parameters
/// - No support for pointer types, structs, arrays, or other complex types
/// - No interface security (authentication/authorization) support
/// - No SEH exception handling
///
/// # Panics
///
/// The macro will fail to compile if:
///
/// - The trait contains non-function items
/// - A method uses `self` receiver
/// - An unsupported type is used in parameters or return values
/// - The GUID format is invalid
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
    let t: syn::ItemTrait = syn::parse2(input)?;

    let mut methods = vec![];
    for item in t.items {
        let TraitItem::Fn(func) = item else {
            return Err(syn::Error::new_spanned(
                input_clone,
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
                    input_clone,
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
        uuid: attrs.guid,
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
