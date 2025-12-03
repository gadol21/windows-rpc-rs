use std::boxed::Box;
use windows::Win32::System::Rpc::{
    RPC_CLIENT_INTERFACE, RPC_DISPATCH_TABLE, RPC_SYNTAX_IDENTIFIER,
};
use windows::core::{GUID, HSTRING, PCWSTR};
use windows_rpc::client_binding::ClientBinding;
use windows_sys::Win32::System::Rpc::{
    MIDL_SERVER_INFO, MIDL_STUB_DESC, MIDL_STUBLESS_PROXY_INFO, MIDL_SYNTAX_INFO, NdrClientCall3,
};
const HELLO_GUID: GUID = GUID::from_u128(162958985766169336398896533731852842919u128);
const RPC_TRANSFER_SYNTAX_2_0: RPC_SYNTAX_IDENTIFIER = RPC_SYNTAX_IDENTIFIER {
    SyntaxGUID: GUID::from_u128(0x8A885D04_1CEB_11C9_9FE8_08002B104860),
    SyntaxVersion: windows::Win32::System::Rpc::RPC_VERSION {
        MajorVersion: 2,
        MinorVersion: 0,
    },
};
struct Hello {
    binding: ClientBinding,
    proxy_info: Box<MIDL_STUBLESS_PROXY_INFO>,
    stub_desc: Box<MIDL_STUB_DESC>,
    syntax_info_array: Box<[MIDL_SYNTAX_INFO; 2]>,
    client_interface: Box<RPC_CLIENT_INTERFACE>,
    iface_handle: Box<*mut RPC_CLIENT_INTERFACE>,
    rpc_transfer_syntax_ndr: Box<RPC_SYNTAX_IDENTIFIER>,
    rpc_transfer_syntax_ndr64: Box<RPC_SYNTAX_IDENTIFIER>,
    type_format: Box<[u8; 3usize]>,
    proc_header: Box<[u8; 31usize]>,
    format_offsets: Box<[u16; 1usize]>,
    ndr64_type_format: Box<[u8; 0usize]>,
    ndr64_proc_buffer: Box<[u8; 32usize]>,
    ndr64_proc_table: Box<[*const u8; 1usize]>,
    auto_bind_handle: Box<*mut core::ffi::c_void>,
}
impl Hello {
    pub fn new(binding: ClientBinding) -> Self {
        let mut auto_bind_handle = Box::new(std::ptr::null_mut());
        let mut type_format: Box<[u8; 3usize]> = Box::new([0u8, 0u8, 0u8]);
        let mut proc_header: Box<[u8; 31usize]> = Box::new([
            0u8, 72u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 50u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 64u8, 0u8, 10u8, 1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        ]);
        let mut format_offsets: Box<[u16; 1usize]> = Box::new([0u16]);
        let mut ndr64_type_format: Box<[u8; 0usize]> = Box::new([]);
        let mut ndr64_proc_buffer: Box<[u8; 32usize]> = Box::new([
            64u8, 0u8, 0u8, 1u8, 8u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 8u8, 0u8, 114u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        ]);
        let ndr64_proc_table: Box<[*const u8; 1usize]> = {
            let base_ptr = ndr64_proc_buffer.as_ptr();
            Box::new([unsafe { base_ptr.add(0usize) }])
        };
        let mut rpc_transfer_syntax_ndr = Box::new(RPC_SYNTAX_IDENTIFIER {
            SyntaxGUID: windows::core::GUID::from_u128(0x8A885D04_1CEB_11C9_9FE8_08002B104860),
            SyntaxVersion: windows::Win32::System::Rpc::RPC_VERSION {
                MajorVersion: 2,
                MinorVersion: 0,
            },
        });
        let mut rpc_transfer_syntax_ndr64 = Box::new(RPC_SYNTAX_IDENTIFIER {
            SyntaxGUID: windows::core::GUID::from_u128(150789598580421593471262320175817215030u128),
            SyntaxVersion: windows::Win32::System::Rpc::RPC_VERSION {
                MajorVersion: 1,
                MinorVersion: 0,
            },
        });
        let mut iface_handle = Box::new(std::ptr::null_mut());
        let mut syntax_info_array = Box::new([
            MIDL_SYNTAX_INFO {
                TransferSyntax: windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER {
                    SyntaxGUID: windows_sys::core::GUID::from_u128(
                        0x8A885D04_1CEB_11C9_9FE8_08002B104860,
                    ),
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
            MIDL_SYNTAX_INFO {
                TransferSyntax: windows_sys::Win32::System::Rpc::RPC_SYNTAX_IDENTIFIER {
                    SyntaxGUID: windows_sys::core::GUID::from_u128(
                        150789598580421593471262320175817215030u128,
                    ),
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
            ProxyServerInfo: std::ptr::null_mut(),
            pExprInfo: std::ptr::null(),
        });
        let mut proxy_info = Box::new(MIDL_STUBLESS_PROXY_INFO {
            pStubDesc: &raw mut *stub_desc,
            ProcFormatString: proc_header.as_mut_ptr(),
            FormatStringOffset: format_offsets.as_mut_ptr(),
            pTransferSyntax: unsafe { std::mem::transmute(&raw mut *rpc_transfer_syntax_ndr) },
            nCount: 2,
            pSyntaxInfo: syntax_info_array.as_mut_ptr(),
        });
        stub_desc.ProxyServerInfo = &raw mut *proxy_info as _;
        let mut client_interface = Box::new(RPC_CLIENT_INTERFACE {
            Length: std::mem::size_of::<RPC_CLIENT_INTERFACE>() as u32,
            InterfaceId: RPC_SYNTAX_IDENTIFIER {
                SyntaxGUID: HELLO_GUID,
                SyntaxVersion: windows::Win32::System::Rpc::RPC_VERSION {
                    MajorVersion: 0u16,
                    MinorVersion: 0u16,
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
    pub fn WriteSomething(&self) {
        unsafe {
            NdrClientCall3(
                &raw const *self.proxy_info as _,
                0u32,
                std::ptr::null_mut(),
                self.binding.handle(),
            );
        }
    }
}

#[test]
fn test() {
    let client = Hello::new(
        ClientBinding::new(
            windows_rpc::client_binding::ProtocolSequence::Alpc,
            "foobar",
        )
        .unwrap(),
    );

    //client.SaySomething("hey", "bye");
    client.WriteSomething();
}
