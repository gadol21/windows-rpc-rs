// NDR format flags and constants
#[allow(non_upper_case_globals)]
pub const Oi_HAS_RPCFLAGS: u8 = 8;
#[allow(non_upper_case_globals)]
pub const Oi_USE_NEW_INIT_ROUTINES: u8 = 0x40;
pub const FC_BIND_PRIMITIVE: u8 = 0x32;
pub const INTERPRETER_OPT_FLAGS2_NEW_CORRELATION_DESCRIPTOR: u8 = 1;
pub const INTERPRETER_OPT_FLAGS2_RANGE_ON_CONFORMANCE: u8 = 0x40;
pub const PARAM_ATTRIBUTES_MUST_SIZE: u16 = 0x1;
pub const PARAM_ATTRIBUTES_MUST_FREE: u16 = 0x2;
pub const PARAM_ATTRIBUTES_IS_IN: u16 = 0x8;
pub const PARAM_ATTRIBUTES_IS_OUT: u16 = 0x10;
pub const PARAM_ATTRIBUTES_IS_RETURN: u16 = 0x20;
pub const PARAM_ATTRIBUTES_IS_BASE_TYPE: u16 = 0x40;
pub const PARAM_ATTRIBUTES_IS_BY_VALUE: u16 = 0x80;
pub const PARAM_ATTRIBUTES_IS_SIMPLE_REF: u16 = 0x100;
// Following consts can be mixed to create 8 + 16 + 32 = 56 bytes.
pub const PARAM_ATTRIBUTES_SERVER_ALLOC_SIZE_8: u16 = 0x2000;
pub const PARAM_ATTRIBUTES_SERVER_ALLOC_SIZE_16: u16 = 0x4000;
pub const PARAM_ATTRIBUTES_SERVER_ALLOC_SIZE_32: u16 = 0x8000;
// Type format string constants
pub const FC_RP: u8 = 0x11; // Reference pointer
pub const FC_UP: u8 = 0x12; // Unique pointer
pub const FC_C_CSTRING: u8 = 0x22; // Conformant character string
pub const FC_C_WSTRING: u8 = 0x25; // Conformant wide character string (unicode)
pub const FC_PAD: u8 = 0x5c; // Padding
pub const FC_SIMPLE_POINTER: u8 = 0x8; // Simple pointer flag

// Procedure flags
pub const OI2_CLIENT_MUST_SIZE: u8 = 0x02;

// Transfer Syntax GUIDs
pub const RPC_TRANSFER_SYNTAX_NDR_GUID: u128 = 0x8A885D04_1CEB_11C9_9FE8_08002B104860;
pub const RPC_TRANSFER_SYNTAX_NDR64_GUID: u128 = 0x71710533_beba_4937_8319_b5dbef9ccc36;

// NDR64 Format Codes (for base types)
pub const NDR64_FC_INT8: u8 = 0x10;
pub const NDR64_FC_INT16: u8 = 0x04;
pub const NDR64_FC_INT32: u8 = 0x05;
pub const NDR64_FC_INT64: u8 = 0x07;
pub const NDR64_FC_CONF_WCHAR_STRING: u8 = 0x64; // Conformant wide character string

// NDR64 Parameter Attributes
pub const NDR64_IS_IN: u16 = 0x0008;
pub const NDR64_IS_OUT: u16 = 0x0010;
pub const NDR64_IS_RETURN: u16 = 0x0020;
pub const NDR64_IS_BASE_TYPE: u16 = 0x0040;
pub const NDR64_IS_BY_VALUE: u16 = 0x0080;
pub const NDR64_MUST_SIZE: u16 = 0x0001;
pub const NDR64_MUST_FREE: u16 = 0x0002;
pub const NDR64_IS_SIMPLE_REF: u16 = 0x0100;

// NDR64 Procedure flags
pub const NDR64_PROC_CLIENT_MUST_SIZE: u32 = 0x00040000;
pub const NDR64_PROC_SERVER_MUST_SIZE: u32 = 0x01000000;

// OI2 flags (INTERPRETER_OPT_FLAGS)
pub const OI2_SERVER_MUST_SIZE: u8 = 0x01;

// MIDL_STUB_DESC values
pub const MIDL_STUB_DESC_CHECK_BOUNDS: i32 = 1;
pub const MIDL_STUB_DESC_VERSION: u32 = 0x60001;
pub const MIDL_STUB_DESC_MIDL_VERSION: u32 = 0x8010274;
pub const MIDL_STUB_DESC_M_FLAGS: usize = 0x2000001;

// RPC_CLIENT_INTERFACE flags
pub const RPC_CLIENT_INTERFACE_FLAGS: i32 = 0x02000000;
