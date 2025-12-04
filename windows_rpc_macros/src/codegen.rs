use quote::{format_ident, quote};

use crate::constants::RPC_TRANSFER_SYNTAX_NDR64_GUID;
use crate::ndr::{generate_proc_header, generate_type_format_string};
use crate::ndr64::{generate_ndr64_proc_buffer_code, generate_ndr64_type_format};
use crate::types::{Interface, Method, Parameter, Type};

fn generate_parameter(param: &Parameter) -> proc_macro2::TokenStream {
    let param_name = format_ident!("{}", param.name);
    let param_type = param.r#type.to_rust_type();
    quote! {
        #param_name: #param_type
    }
}

fn generate_method(method: (usize, &Method)) -> proc_macro2::TokenStream {
    let (method_index, method) = method;
    let method_index = method_index as u32;
    let method_name = format_ident!("{}", method.name);
    let parameters = method.parameters.iter().map(generate_parameter);
    let parameters_propagation = method.parameters.iter().map(|param| {
        param
            .r#type
            .rust_type_to_abi(format_ident!("{}", param.name))
    });

    let (method_suffix, return_suffix) = if let Some(rtype) = &method.return_type
        && matches!(rtype, Type::Simple(_))
    {
        let rtype = rtype.to_rust_type();
        (
            quote! {
                .Simple as #rtype
            },
            quote! {
                -> #rtype
            },
        )
    } else {
        (quote! { ; }, quote! {})
    };

    quote! {
        pub fn #method_name(&self, #(#parameters),*) #return_suffix {
            unsafe {
                NdrClientCall3(&raw const *self.proxy_info as _, #method_index, std::ptr::null_mut(), self.binding.handle(), #(#parameters_propagation),*)#method_suffix
            }
        }
    }
}

pub fn compile_client(interface: Interface) -> proc_macro2::TokenStream {
    let rpc_client_name = format_ident!("{}Client", interface.name);
    let interface_guid_name = format_ident!("{}_GUID", interface.name.to_uppercase());
    let interface_guid = interface.uuid.to_u128();
    let interface_version_major = interface.version.major;
    let interface_version_minor = interface.version.minor;
    let methods = interface.methods.iter().enumerate().map(generate_method);

    // Generate NDR format strings
    let (type_format, type_offsets) = generate_type_format_string(&interface);
    let type_format_len = type_format.len();

    // Generate proc header with type offsets
    let (proc_header, format_offsets) = generate_proc_header(&interface, &type_offsets);
    let proc_header_len = proc_header.len();
    let format_offsets_len = format_offsets.len();

    // Generate NDR64 format structures
    let ndr64_type_format = generate_ndr64_type_format(&interface);
    let ndr64_type_format_len = ndr64_type_format.len();

    // Generate code to build proc buffer at runtime
    let ndr64_proc_buffer_construction = generate_ndr64_proc_buffer_code(&interface);
    let ndr64_proc_table_len = interface.methods.len();
    let proc_table_indices: Vec<_> = (0..ndr64_proc_table_len).collect();

    quote! {
        use std::boxed::Box;
        use windows::core::{GUID, HSTRING, PCWSTR};
        use windows::Win32::System::Rpc::{
            RPC_CLIENT_INTERFACE, RPC_DISPATCH_TABLE, RPC_SYNTAX_IDENTIFIER
        };
        use windows_sys::Win32::System::Rpc::{
            MIDL_SERVER_INFO, MIDL_STUB_DESC, MIDL_STUBLESS_PROXY_INFO, NdrClientCall3,
            MIDL_SYNTAX_INFO
        };
        use windows_rpc::client_binding::ClientBinding;

        const #interface_guid_name: GUID = GUID::from_u128(#interface_guid);

        // FIXME: move to helper module
        // RPC transfer syntax identifier for NDR
        const RPC_TRANSFER_SYNTAX_2_0: RPC_SYNTAX_IDENTIFIER = RPC_SYNTAX_IDENTIFIER {
            SyntaxGUID: GUID::from_u128(0x8A885D04_1CEB_11C9_9FE8_08002B104860),
            SyntaxVersion: windows::Win32::System::Rpc::RPC_VERSION {
                MajorVersion: 2,
                MinorVersion: 0,
            },
        };

        struct #rpc_client_name {
            binding: ClientBinding,
            // metadata needed for RPC calls
            proxy_info: Box<MIDL_STUBLESS_PROXY_INFO>,
            stub_desc: Box<MIDL_STUB_DESC>,
            syntax_info_array: Box<[MIDL_SYNTAX_INFO; 2]>,
            client_interface: Box<RPC_CLIENT_INTERFACE>,
            iface_handle: Box<*mut RPC_CLIENT_INTERFACE>,
            rpc_transfer_syntax_ndr: Box<RPC_SYNTAX_IDENTIFIER>,
            rpc_transfer_syntax_ndr64: Box<RPC_SYNTAX_IDENTIFIER>,
            type_format: Box<[u8; #type_format_len]>,
            proc_header: Box<[u8; #proc_header_len]>,
            format_offsets: Box<[u16; #format_offsets_len]>,
            // NDR64 format data (contiguous memory)
            ndr64_type_format: Box<[u8; #ndr64_type_format_len]>,
            ndr64_proc_buffer: Box<Vec<u8>>,  // Built at runtime, variable size
            ndr64_proc_table: Box<[*const u8; #ndr64_proc_table_len]>,
            auto_bind_handle: Box<*mut core::ffi::c_void>,
        }

        impl #rpc_client_name {
            pub fn new(binding: ClientBinding) -> Self {
                let mut auto_bind_handle = Box::new(std::ptr::null_mut());
                let mut type_format: Box<[u8; #type_format_len]> = Box::new([#(#type_format),*]);
                let mut proc_header: Box<[u8; #proc_header_len]> = Box::new([#(#proc_header),*]);
                let mut format_offsets: Box<[u16; #format_offsets_len]> = Box::new([#(#format_offsets),*]);

                // Initialize NDR64 data structures
                let ndr64_type_format: Box<[u8; #ndr64_type_format_len]> =
                    Box::new([#(#ndr64_type_format),*]);

                // Build proc buffer at runtime (so pointers to ndr64_type_format are valid)
                let (ndr64_proc_buffer_data, proc_table_offsets) = #ndr64_proc_buffer_construction;

                let ndr64_proc_buffer = Box::new(ndr64_proc_buffer_data);

                // Build Ndr64ProcTable - array of pointers into proc_buffer
                let ndr64_proc_table: Box<[*const u8; #ndr64_proc_table_len]> = {
                    let base_ptr = ndr64_proc_buffer.as_ptr();
                    Box::new([
                        #(unsafe { base_ptr.add(proc_table_offsets[#proc_table_indices]) }),*
                    ])
                };

                let mut rpc_transfer_syntax_ndr = Box::new(RPC_SYNTAX_IDENTIFIER {
                    SyntaxGUID: windows::core::GUID::from_u128(0x8A885D04_1CEB_11C9_9FE8_08002B104860),
                    SyntaxVersion: windows::Win32::System::Rpc::RPC_VERSION {
                        MajorVersion: 2,
                        MinorVersion: 0,
                    },
                });

                // Create NDR64 transfer syntax
                let rpc_transfer_syntax_ndr64 = Box::new(RPC_SYNTAX_IDENTIFIER {
                    SyntaxGUID: windows::core::GUID::from_u128(#RPC_TRANSFER_SYNTAX_NDR64_GUID),
                    SyntaxVersion: windows::Win32::System::Rpc::RPC_VERSION {
                        MajorVersion: 1,
                        MinorVersion: 0,
                    },
                });

                let mut iface_handle = Box::new(std::ptr::null_mut());

                // Create array of two syntax infos
                let mut syntax_info_array = Box::new([
                    // NDR 2.0 syntax info (index 0)
                    MIDL_SYNTAX_INFO {
                        TransferSyntax: windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER {
                            SyntaxGUID: windows_sys::core::GUID::from_u128(0x8A885D04_1CEB_11C9_9FE8_08002B104860),
                            SyntaxVersion: windows_sys::Win32::System::Rpc::RPC_VERSION {
                                MajorVersion: 2,
                                MinorVersion: 0,
                            },
                        },
                        DispatchTable: std::ptr::null_mut(),
                        ProcString: proc_header.as_mut_ptr(),
                        FmtStringOffset: format_offsets.as_ptr(),
                        TypeString: type_format.as_mut_ptr(),
                        aUserMarshalQuadruple: std::ptr::null(),
                        pMethodProperties: std::ptr::null(),
                        pReserved2: 0,
                    },
                    // NDR64 1.0 syntax info (index 1)
                    MIDL_SYNTAX_INFO {
                        TransferSyntax: windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER {
                            SyntaxGUID: windows_sys::core::GUID::from_u128(#RPC_TRANSFER_SYNTAX_NDR64_GUID),
                            SyntaxVersion: windows_sys::Win32::System::Rpc::RPC_VERSION {
                                MajorVersion: 1,
                                MinorVersion: 0,
                            },
                        },
                        DispatchTable: std::ptr::null_mut(),
                        ProcString: std::ptr::null_mut(),
                        FmtStringOffset: ndr64_proc_table.as_ptr() as *const u16,
                        TypeString: std::ptr::null_mut(),
                        aUserMarshalQuadruple: std::ptr::null(),
                        pMethodProperties: std::ptr::null(),
                        pReserved2: 0,
                    },
                ]);
                let mut stub_desc = Box::new(MIDL_STUB_DESC {
                    // Will be filled later
                    RpcInterfaceInformation: std::ptr::null_mut(),
                    pfnAllocate: Some(windows_rpc::alloc::midl_alloc),
                    pfnFree: Some(windows_rpc::alloc::midl_free),
                    IMPLICIT_HANDLE_INFO: windows_sys::Win32::System::Rpc::MIDL_STUB_DESC_0 {
                        pAutoHandle: &raw mut *auto_bind_handle,
                    },
                    apfnNdrRundownRoutines: std::ptr::null(),
                    aGenericBindingRoutinePairs: std::ptr::null(),
                    apfnExprEval: std::ptr::null(),
                    aXmitQuintuple: std::ptr::null(),
                    pFormatTypes: type_format.as_ptr(),
                    fCheckBounds: 1,
                    Version: 0x60001,
                    pMallocFreeStruct: std::ptr::null_mut(),
                    MIDLVersion: 0x8010274,
                    CommFaultOffsets: std::ptr::null(),
                    aUserMarshalQuadruple: std::ptr::null(),
                    NotifyRoutineTable: std::ptr::null(),
                    mFlags: 0x2000001,
                    CsRoutineTables: std::ptr::null(),
                    // Will be filled later
                    ProxyServerInfo: std::ptr::null_mut(),
                    pExprInfo: std::ptr::null(),
                });
                // Update proxy info to point to dual syntax array
                let mut proxy_info = Box::new(MIDL_STUBLESS_PROXY_INFO {
                    pStubDesc: &raw mut *stub_desc,
                    ProcFormatString: proc_header.as_mut_ptr(),
                    FormatStringOffset: format_offsets.as_mut_ptr(),
                    pTransferSyntax: unsafe { std::mem::transmute(&raw mut *rpc_transfer_syntax_ndr) },
                    nCount: 2,  // Changed from 1 to 2!
                    pSyntaxInfo: syntax_info_array.as_mut_ptr(),
                });
                // Circular dependency fixup
                stub_desc.ProxyServerInfo = &raw mut *proxy_info as _;

                let mut client_interface= Box::new(RPC_CLIENT_INTERFACE {
                    Length: std::mem::size_of::<RPC_CLIENT_INTERFACE>() as u32,
                    InterfaceId: RPC_SYNTAX_IDENTIFIER {
                        SyntaxGUID: #interface_guid_name,
                        SyntaxVersion: windows::Win32::System::Rpc::RPC_VERSION {
                            MajorVersion: #interface_version_major,
                            MinorVersion: #interface_version_minor,
                        },
                    },
                    TransferSyntax: RPC_SYNTAX_IDENTIFIER {
                        SyntaxGUID: GUID::from_u128(0x8A885D04_1CEB_11C9_9FE8_08002B104860),
                        SyntaxVersion: windows::Win32::System::Rpc::RPC_VERSION {
                            MajorVersion: 2,
                            MinorVersion: 0,
                        },
                    },
                    DispatchTable: std::ptr::null_mut(),
                    RpcProtseqEndpointCount: 0,
                    RpcProtseqEndpoint: std::ptr::null_mut(),
                    Reserved: 0,
                    InterpreterInfo: &raw const *proxy_info as _,
                    Flags: 0x02000000,
                });
                *iface_handle = &raw mut *client_interface;
                stub_desc.RpcInterfaceInformation = &raw mut *client_interface as _;

                Self {
                    binding,
                    proxy_info,
                    client_interface,
                    stub_desc,
                    syntax_info_array,
                    iface_handle,
                    rpc_transfer_syntax_ndr,
                    rpc_transfer_syntax_ndr64,
                    format_offsets,
                    proc_header,
                    type_format,
                    ndr64_type_format,
                    ndr64_proc_buffer,
                    ndr64_proc_table,
                    auto_bind_handle,
                }
            }

            #(#methods)*
        }
    }
}
