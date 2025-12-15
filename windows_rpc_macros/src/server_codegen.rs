use quote::{format_ident, quote};

use crate::constants::{
    MIDL_STUB_DESC_CHECK_BOUNDS, MIDL_STUB_DESC_M_FLAGS, MIDL_STUB_DESC_MIDL_VERSION,
    MIDL_STUB_DESC_VERSION, RPC_TRANSFER_SYNTAX_NDR_GUID, RPC_TRANSFER_SYNTAX_NDR64_GUID,
};
use crate::ndr::{generate_proc_header, generate_type_format_string};
use crate::ndr64::{generate_ndr64_proc_buffer_code, generate_ndr64_type_format};
use crate::types::Interface;

use crate::types::Type;

/// Generate the server implementation trait that users will implement
fn generate_server_trait(interface: &Interface) -> proc_macro2::TokenStream {
    let trait_name = format_ident!("{}ServerImpl", interface.name);

    let methods: Vec<_> = interface
        .methods
        .iter()
        .map(|method| {
            let method_name = format_ident!("{}", method.name);
            let params: Vec<_> = method
                .parameters
                .iter()
                .map(|param| {
                    let param_name = format_ident!("{}", param.name);
                    let param_type = param.r#type.to_rust_type();
                    quote! { #param_name: #param_type }
                })
                .collect();

            let return_type = if let Some(rtype) = &method.return_type {
                // Use to_rust_return_type for return values (String instead of &str)
                let rtype_tokens = rtype.to_rust_return_type();
                quote! { -> #rtype_tokens }
            } else {
                quote! {}
            };

            quote! {
                fn #method_name(&self, #(#params),*) #return_type;
            }
        })
        .collect();

    quote! {
        pub trait #trait_name: Send + Sync + 'static {
            #(#methods)*
        }
    }
}

/// Generate extern "C" wrapper functions for each method
fn generate_wrapper_functions(interface: &Interface) -> proc_macro2::TokenStream {
    let trait_name = format_ident!("{}ServerImpl", interface.name);

    let wrappers: Vec<_> = interface
        .methods
        .iter()
        .map(|method| {
            let wrapper_name = format_ident!("__{}__{}_wrapper", interface.name, method.name);
            let method_name = format_ident!("{}", method.name);
            let has_string_return = matches!(method.return_type, Some(Type::String));

            // Generate FFI parameter types (PCWSTR for strings, native types for others)
            let mut ffi_params: Vec<_> = method
                .parameters
                .iter()
                .map(|param| {
                    let param_name = format_ident!("{}", param.name);
                    let param_type = if matches!(param.r#type, Type::String) {
                        quote! { windows::core::PCWSTR }
                    } else {
                        param.r#type.to_rust_type()
                    };
                    quote! { #param_name: #param_type }
                })
                .collect();

            // Add out string parameter if function returns String
            if has_string_return {
                ffi_params.push(quote! { __out_string: *mut *mut u16 });
            }

            // Generate string conversions for string parameters
            let string_conversions: Vec<_> = method
                .parameters
                .iter()
                .filter(|p| matches!(p.r#type, Type::String))
                .map(|param| {
                    let param_name = format_ident!("{}", param.name);
                    let converted_name = format_ident!("__{}_converted", param.name);
                    quote! {
                        let #converted_name = unsafe { #param_name.to_string().unwrap() };
                    }
                })
                .collect();

            // Generate parameter names for the trait method call (converted names for strings)
            let param_names: Vec<_> = method
                .parameters
                .iter()
                .map(|param| {
                    if matches!(param.r#type, Type::String) {
                        let converted_name = format_ident!("__{}_converted", param.name);
                        quote! { #converted_name.as_str() }
                    } else {
                        let param_name = format_ident!("{}", param.name);
                        quote! { #param_name }
                    }
                })
                .collect();

            // Generate the wrapper body based on return type
            match &method.return_type {
                Some(Type::Simple(_)) => {
                    let rtype_tokens = method.return_type.as_ref().unwrap().to_rust_return_type();
                    quote! {
                        extern "C" fn #wrapper_name(binding_handle: *const std::ffi::c_void, #(#ffi_params),*) -> #rtype_tokens {
                            #(#string_conversions)*
                            windows_rpc::server::with_context::<dyn #trait_name, _, _>(|impl_| {
                                impl_.#method_name(#(#param_names),*)
                            })
                        }
                    }
                }
                Some(Type::String) => {
                    // For string return, we don't return anything directly - we write to the out param
                    quote! {
                        extern "C" fn #wrapper_name(binding_handle: *const std::ffi::c_void, #(#ffi_params),*) {
                            #(#string_conversions)*
                            let __result = windows_rpc::server::with_context::<dyn #trait_name, _, _>(|impl_| {
                                impl_.#method_name(#(#param_names),*)
                            });

                            // Convert the Rust String to a wide string and allocate with midl_user_allocate
                            unsafe {
                                // Convert to UTF-16 with null terminator
                                let wide: Vec<u16> = __result.encode_utf16().chain(std::iter::once(0)).collect();
                                let byte_len = wide.len() * std::mem::size_of::<u16>();

                                // Allocate memory using MIDL allocator
                                let ptr = windows_rpc::alloc::midl_alloc(byte_len) as *mut u16;
                                if !ptr.is_null() {
                                    // Copy the wide string to the allocated memory
                                    std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                                }

                                // Write the pointer to the out parameter
                                *__out_string = ptr;
                            }
                        }
                    }
                }
                None => {
                    quote! {
                        extern "C" fn #wrapper_name(binding_handle: *const std::ffi::c_void, #(#ffi_params),*) {
                            #(#string_conversions)*
                            windows_rpc::server::with_context::<dyn #trait_name, _, _>(|impl_| {
                                impl_.#method_name(#(#param_names),*)
                            })
                        }
                    }
                }
            }
        })
        .collect();

    quote! {
        #(#wrappers)*
    }
}

/// Generate the dispatch table initialization
fn generate_dispatch_table_init(interface: &Interface) -> proc_macro2::TokenStream {
    let method_count = interface.methods.len();

    // For NDR 2.0, all entries point to NdrServerCall2
    let ndr_entries = (0..method_count).map(|_| {
        quote! {
            std::option::Option::Some(windows_sys::Win32::System::Rpc::NdrServerCall2)
        }
    });

    // For NDR64, all entries point to NdrServerCallAll
    let ndr64_entries = (0..method_count).map(|_| {
        quote! {
            std::option::Option::Some(windows_sys::Win32::System::Rpc::NdrServerCallAll)
        }
    });

    quote! {
        let dispatch_functions_ndr: std::boxed::Box<[windows_sys::Win32::System::Rpc::RPC_DISPATCH_FUNCTION; #method_count]> =
            std::boxed::Box::new([#(#ndr_entries),*]);

        let dispatch_functions_ndr64: std::boxed::Box<[windows_sys::Win32::System::Rpc::RPC_DISPATCH_FUNCTION; #method_count]> =
            std::boxed::Box::new([#(#ndr64_entries),*]);
    }
}

/// Generate the server routine table initialization
fn generate_server_routine_table(interface: &Interface) -> proc_macro2::TokenStream {
    let method_count = interface.methods.len();

    let wrapper_casts: Vec<_> = interface
        .methods
        .iter()
        .map(|method| {
            let wrapper_name = format_ident!("__{}__{}_wrapper", interface.name, method.name);
            quote! {
                unsafe {
                    std::mem::transmute::<
                        *const (),
                        windows_sys::Win32::System::Rpc::SERVER_ROUTINE
                    >(#wrapper_name as *const ())
                }
            }
        })
        .collect();

    quote! {
        let server_routines: std::boxed::Box<[windows_sys::Win32::System::Rpc::SERVER_ROUTINE; #method_count]> =
            std::boxed::Box::new([#(#wrapper_casts),*]);
    }
}

pub fn compile_server(interface: &Interface) -> proc_macro2::TokenStream {
    let rpc_server_name = format_ident!("{}Server", interface.name);
    let trait_name = format_ident!("{}ServerImpl", interface.name);
    let interface_guid_name = format_ident!("{}_GUID", interface.name.to_uppercase());
    let interface_version_major = interface.version.major;
    let interface_version_minor = interface.version.minor;

    // Generate format strings (reused from client)
    let (type_format, type_offsets) = generate_type_format_string(interface);
    let type_format_len = type_format.len();

    let (proc_header, format_offsets) = generate_proc_header(interface, &type_offsets);
    let proc_header_len = proc_header.len();
    let format_offsets_len = format_offsets.len();

    let ndr64_type_format = generate_ndr64_type_format(interface);
    let ndr64_type_format_len = ndr64_type_format.len();

    let ndr64_proc_buffer_construction = generate_ndr64_proc_buffer_code(interface);
    let ndr64_proc_table_len = interface.methods.len();
    let proc_table_indices: Vec<_> = (0..ndr64_proc_table_len).collect();

    let method_count = interface.methods.len();

    // Generate components
    let server_trait = generate_server_trait(interface);
    let wrapper_functions = generate_wrapper_functions(interface);
    let dispatch_table_init = generate_dispatch_table_init(interface);
    let server_routine_table = generate_server_routine_table(interface);

    quote! {
        #server_trait

        #wrapper_functions

        pub struct #rpc_server_name {
            // RPC metadata structures
            server_interface: std::boxed::Box<windows_sys::Win32::System::Rpc::RPC_SERVER_INTERFACE>,
            server_info: std::boxed::Box<windows_sys::Win32::System::Rpc::MIDL_SERVER_INFO>,
            stub_desc: std::boxed::Box<windows_sys::Win32::System::Rpc::MIDL_STUB_DESC>,
            dispatch_table_ndr: std::boxed::Box<windows_sys::Win32::System::Rpc::RPC_DISPATCH_TABLE>,
            dispatch_table_ndr64: std::boxed::Box<windows_sys::Win32::System::Rpc::RPC_DISPATCH_TABLE>,
            dispatch_functions_ndr: std::boxed::Box<[windows_sys::Win32::System::Rpc::RPC_DISPATCH_FUNCTION; #method_count]>,
            dispatch_functions_ndr64: std::boxed::Box<[windows_sys::Win32::System::Rpc::RPC_DISPATCH_FUNCTION; #method_count]>,
            server_routines: std::boxed::Box<[windows_sys::Win32::System::Rpc::SERVER_ROUTINE; #method_count]>,
            syntax_info_array: std::boxed::Box<[windows_sys::Win32::System::Rpc::MIDL_SYNTAX_INFO; 2]>,
            rpc_transfer_syntax_ndr: std::boxed::Box<windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER>,
            rpc_transfer_syntax_ndr64: std::boxed::Box<windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER>,

            // Format strings
            type_format: std::boxed::Box<[u8; #type_format_len]>,
            proc_header: std::boxed::Box<[u8; #proc_header_len]>,
            format_offsets: std::boxed::Box<[u16; #format_offsets_len]>,
            ndr64_type_format: std::boxed::Box<[u8; #ndr64_type_format_len]>,
            ndr64_proc_buffer: std::boxed::Box<std::vec::Vec<u8>>,
            ndr64_proc_table: std::boxed::Box<[*const u8; #ndr64_proc_table_len]>,
            auto_bind_handle: std::boxed::Box<*mut std::ffi::c_void>,

            // Server state
            implementation: std::boxed::Box<dyn #trait_name>,
            binding: std::option::Option<windows_rpc::server_binding::ServerBinding>,
        }

        impl #rpc_server_name {
            pub fn new<T: #trait_name>(implementation: T) -> Self {
                let implementation = std::boxed::Box::new(implementation) as std::boxed::Box<dyn #trait_name>;
                let mut auto_bind_handle = std::boxed::Box::new(std::ptr::null_mut());

                // Initialize format strings
                let mut type_format: std::boxed::Box<[u8; #type_format_len]> = std::boxed::Box::new([#(#type_format),*]);
                let mut proc_header: std::boxed::Box<[u8; #proc_header_len]> = std::boxed::Box::new([#(#proc_header),*]);
                let mut format_offsets: std::boxed::Box<[u16; #format_offsets_len]> = std::boxed::Box::new([#(#format_offsets),*]);

                let ndr64_type_format: std::boxed::Box<[u8; #ndr64_type_format_len]> =
                    std::boxed::Box::new([#(#ndr64_type_format),*]);

                let (ndr64_proc_buffer_data, proc_table_offsets) = #ndr64_proc_buffer_construction;
                let ndr64_proc_buffer = std::boxed::Box::new(ndr64_proc_buffer_data);

                let ndr64_proc_table: std::boxed::Box<[*const u8; #ndr64_proc_table_len]> = {
                    let base_ptr = ndr64_proc_buffer.as_ptr();
                    std::boxed::Box::new([
                        #(unsafe { base_ptr.add(proc_table_offsets[#proc_table_indices]) }),*
                    ])
                };

                // Create transfer syntax identifiers
                let mut rpc_transfer_syntax_ndr = std::boxed::Box::new(windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER {
                    SyntaxGUID: windows_sys::core::GUID::from_u128(#RPC_TRANSFER_SYNTAX_NDR_GUID),
                    SyntaxVersion: windows_sys::Win32::System::Rpc::RPC_VERSION {
                        MajorVersion: 2,
                        MinorVersion: 0,
                    },
                });

                let mut rpc_transfer_syntax_ndr64 = std::boxed::Box::new(windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER {
                    SyntaxGUID: windows_sys::core::GUID::from_u128(#RPC_TRANSFER_SYNTAX_NDR64_GUID),
                    SyntaxVersion: windows_sys::Win32::System::Rpc::RPC_VERSION {
                        MajorVersion: 1,
                        MinorVersion: 0,
                    },
                });

                // Create dispatch tables and routine tables
                #dispatch_table_init
                #server_routine_table

                let mut dispatch_table_ndr = std::boxed::Box::new(windows_sys::Win32::System::Rpc::RPC_DISPATCH_TABLE {
                    DispatchTableCount: #method_count as u32,
                    DispatchTable: std::option::Option::None,
                    Reserved: 0,
                });

                let mut dispatch_table_ndr64 = std::boxed::Box::new(windows_sys::Win32::System::Rpc::RPC_DISPATCH_TABLE {
                    DispatchTableCount: #method_count as u32,
                    DispatchTable: std::option::Option::None,
                    Reserved: 0,
                });

                // Create syntax info array
                let mut syntax_info_array = std::boxed::Box::new([
                    windows_sys::Win32::System::Rpc::MIDL_SYNTAX_INFO {
                        TransferSyntax: windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER {
                            SyntaxGUID: windows_sys::core::GUID::from_u128(#RPC_TRANSFER_SYNTAX_NDR_GUID),
                            SyntaxVersion: windows_sys::Win32::System::Rpc::RPC_VERSION {
                                MajorVersion: 2,
                                MinorVersion: 0,
                            },
                        },
                        DispatchTable: &raw mut *dispatch_table_ndr as *mut _,
                        ProcString: proc_header.as_mut_ptr(),
                        FmtStringOffset: format_offsets.as_ptr(),
                        TypeString: type_format.as_mut_ptr(),
                        aUserMarshalQuadruple: std::ptr::null(),
                        pMethodProperties: std::ptr::null(),
                        pReserved2: 0,
                    },
                    windows_sys::Win32::System::Rpc::MIDL_SYNTAX_INFO {
                        TransferSyntax: windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER {
                            SyntaxGUID: windows_sys::core::GUID::from_u128(#RPC_TRANSFER_SYNTAX_NDR64_GUID),
                            SyntaxVersion: windows_sys::Win32::System::Rpc::RPC_VERSION {
                                MajorVersion: 1,
                                MinorVersion: 0,
                            },
                        },
                        DispatchTable: &raw mut *dispatch_table_ndr64 as *mut _,
                        ProcString: std::ptr::null_mut(),
                        FmtStringOffset: ndr64_proc_table.as_ptr() as *const u16,
                        TypeString: std::ptr::null_mut(),
                        aUserMarshalQuadruple: std::ptr::null(),
                        pMethodProperties: std::ptr::null(),
                        pReserved2: 0,
                    },
                ]);

                // Create stub desc
                let mut stub_desc = std::boxed::Box::new(windows_sys::Win32::System::Rpc::MIDL_STUB_DESC {
                    // Will be filled later
                    RpcInterfaceInformation: std::ptr::null_mut(),
                    pfnAllocate: std::option::Option::Some(windows_rpc::alloc::midl_alloc),
                    pfnFree: std::option::Option::Some(windows_rpc::alloc::midl_free),
                    IMPLICIT_HANDLE_INFO: windows_sys::Win32::System::Rpc::MIDL_STUB_DESC_0 {
                        pAutoHandle: &raw mut *auto_bind_handle,
                    },
                    apfnNdrRundownRoutines: std::ptr::null(),
                    aGenericBindingRoutinePairs: std::ptr::null(),
                    apfnExprEval: std::ptr::null(),
                    aXmitQuintuple: std::ptr::null(),
                    pFormatTypes: type_format.as_ptr(),
                    fCheckBounds: #MIDL_STUB_DESC_CHECK_BOUNDS as _,
                    Version: #MIDL_STUB_DESC_VERSION as _,
                    pMallocFreeStruct: std::ptr::null_mut(),
                    MIDLVersion: #MIDL_STUB_DESC_MIDL_VERSION as _,
                    CommFaultOffsets: std::ptr::null(),
                    aUserMarshalQuadruple: std::ptr::null(),
                    NotifyRoutineTable: std::ptr::null(),
                    mFlags: #MIDL_STUB_DESC_M_FLAGS as _,
                    CsRoutineTables: std::ptr::null(),
                    // Will be filled later
                    ProxyServerInfo: std::ptr::null_mut(),
                    pExprInfo: std::ptr::null(),
                });

                // Create server info
                let mut server_info = std::boxed::Box::new(windows_sys::Win32::System::Rpc::MIDL_SERVER_INFO {
                    pStubDesc: &raw mut *stub_desc,
                    DispatchTable: server_routines.as_ptr() as _,
                    ProcString: proc_header.as_mut_ptr(),
                    FmtStringOffset: format_offsets.as_ptr(),
                    ThunkTable: std::ptr::null(),
                    pTransferSyntax: &raw mut *rpc_transfer_syntax_ndr as *mut _ as *mut _,
                    nCount: 2,
                    pSyntaxInfo: syntax_info_array.as_mut_ptr(),
                });

                // Create server interface
                let interface_guid_u128 = #interface_guid_name.to_u128();
                let mut server_interface = std::boxed::Box::new(windows_sys::Win32::System::Rpc::RPC_SERVER_INTERFACE {
                    Length: std::mem::size_of::<windows_sys::Win32::System::Rpc::RPC_SERVER_INTERFACE>() as u32,
                    InterfaceId: windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER {
                        SyntaxGUID: windows_sys::core::GUID::from_u128(interface_guid_u128),
                        SyntaxVersion: windows_sys::Win32::System::Rpc::RPC_VERSION {
                            MajorVersion: #interface_version_major,
                            MinorVersion: #interface_version_minor,
                        },
                    },
                    TransferSyntax: windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER {
                        SyntaxGUID: windows_sys::core::GUID::from_u128(#RPC_TRANSFER_SYNTAX_NDR_GUID),
                        SyntaxVersion: windows_sys::Win32::System::Rpc::RPC_VERSION {
                            MajorVersion: 2,
                            MinorVersion: 0,
                        },
                    },
                    DispatchTable: &raw mut *dispatch_table_ndr,
                    RpcProtseqEndpointCount: 0,
                    RpcProtseqEndpoint: std::ptr::null_mut(),
                    DefaultManagerEpv: std::ptr::null_mut(),
                    InterpreterInfo: &raw const *server_info as *const _ as *const _,
                    // FIXME: doesn't this need to be 0x06000000?
                    //Flags: 0x04000000, // Support NDR64
                    Flags: 0x06000000,
                });

                // Fixup circular references
                dispatch_table_ndr.DispatchTable = std::option::Option::Some(unsafe {
                    std::mem::transmute::<*const windows_sys::Win32::System::Rpc::RPC_DISPATCH_FUNCTION, _>(
                        dispatch_functions_ndr.as_ptr()
                    )
                });
                dispatch_table_ndr64.DispatchTable = std::option::Option::Some(unsafe {
                    std::mem::transmute::<*const windows_sys::Win32::System::Rpc::RPC_DISPATCH_FUNCTION, _>(
                        dispatch_functions_ndr64.as_ptr()
                    )
                });
                stub_desc.RpcInterfaceInformation = &raw mut *server_interface as *mut _ as *mut _;
                stub_desc.ProxyServerInfo = &raw mut *server_info as _;

                Self {
                    server_interface,
                    server_info,
                    stub_desc,
                    dispatch_table_ndr,
                    dispatch_table_ndr64,
                    dispatch_functions_ndr,
                    dispatch_functions_ndr64,
                    server_routines,
                    syntax_info_array,
                    rpc_transfer_syntax_ndr,
                    rpc_transfer_syntax_ndr64,
                    type_format,
                    proc_header,
                    format_offsets,
                    ndr64_type_format,
                    ndr64_proc_buffer,
                    ndr64_proc_table,
                    auto_bind_handle,
                    implementation,
                    binding: std::option::Option::None,
                }
            }

            pub fn register(&mut self, endpoint: &str) -> std::result::Result<(), windows::core::Error> {
                // Set the implementation context for this thread
                let impl_ptr: *const dyn #trait_name = &*self.implementation as *const _;
                unsafe {
                    windows_rpc::server::set_context(impl_ptr);
                }

                let binding = windows_rpc::server_binding::ServerBinding::new(
                    windows_rpc::ProtocolSequence::Alpc,
                    endpoint,
                    &raw const *self.server_interface as *const _ as *const std::ffi::c_void,
                )?;

                self.binding = std::option::Option::Some(binding);
                self.binding.as_mut().unwrap().register()?;

                std::result::Result::Ok(())
            }

            pub fn listen(&self) -> std::result::Result<(), windows::core::Error> {
                if let std::option::Option::Some(binding) = &self.binding {
                    binding.listen()
                } else {
                    std::result::Result::Err(windows::core::Error::from_hresult(windows::core::HRESULT(-1)))
                }
            }

            pub fn listen_async(&self) -> std::result::Result<(), windows::core::Error> {
                if let std::option::Option::Some(binding) = &self.binding {
                    binding.listen_async()
                } else {
                    std::result::Result::Err(windows::core::Error::from_hresult(windows::core::HRESULT(-1)))
                }
            }

            pub fn stop(&self) -> std::result::Result<(), windows::core::Error> {
                if let std::option::Option::Some(binding) = &self.binding {
                    binding.stop()?;
                }
                windows_rpc::server::clear_context();
                std::result::Result::Ok(())
            }
        }

        impl std::ops::Drop for #rpc_server_name {
            fn drop(&mut self) {
                let _ = self.stop();
            }
        }
    }
}
