use std::collections::HashMap;

use quote::{ToTokens, format_ident, quote};
use syn::{
    FnArg, Ident, LitFloat, LitInt, ReturnType, Token, TraitItem, Type as SynType, parse::Parse,
};
use windows::core::GUID;

/// Parsed attributes for the rpc_interface macro
struct InterfaceAttributes {
    guid: u128,
    version: InterfaceVersion,
}

impl Parse for InterfaceAttributes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut guid: Option<u128> = None;
        let mut version: Option<InterfaceVersion> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let content;
            syn::parenthesized!(content in input);

            match ident.to_string().as_str() {
                "guid" => {
                    let lit: LitInt = content.parse()?;
                    guid = Some(lit.base10_parse::<u128>().map_err(|_| {
                        syn::Error::new_spanned(&lit, "Expected a u128 hex literal for guid")
                    })?);
                }
                "version" => {
                    // Parse version as either "major.minor" float literal or two integers
                    if content.peek(LitFloat) {
                        let lit: LitFloat = content.parse()?;
                        let version_str = lit.to_string();
                        let parts: Vec<&str> = version_str.split('.').collect();
                        if parts.len() != 2 {
                            return Err(syn::Error::new_spanned(
                                &lit,
                                "Expected version format: major.minor",
                            ));
                        }
                        let major: u16 = parts[0].parse().map_err(|_| {
                            syn::Error::new_spanned(&lit, "Invalid major version number")
                        })?;
                        let minor: u16 = parts[1].parse().map_err(|_| {
                            syn::Error::new_spanned(&lit, "Invalid minor version number")
                        })?;
                        version = Some(InterfaceVersion { major, minor });
                    } else if content.peek(LitInt) {
                        // Handle case like version(1) meaning 1.0
                        let major_lit: LitInt = content.parse()?;
                        let major: u16 = major_lit.base10_parse()?;
                        let minor = if content.peek(Token![.]) {
                            content.parse::<Token![.]>()?;
                            let minor_lit: LitInt = content.parse()?;
                            minor_lit.base10_parse()?
                        } else {
                            0
                        };
                        version = Some(InterfaceVersion { major, minor });
                    } else {
                        return Err(syn::Error::new(content.span(), "Expected version number"));
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        &ident,
                        format!("Unknown attribute: {}", other),
                    ));
                }
            }

            // Consume optional comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let guid =
            guid.ok_or_else(|| syn::Error::new(input.span(), "Missing required 'guid' attribute"))?;
        let version = version.unwrap_or_default();

        Ok(InterfaceAttributes { guid, version })
    }
}

#[allow(non_upper_case_globals)]
const Oi_HAS_RPCFLAGS: u8 = 8;
#[allow(non_upper_case_globals)]
const Oi_USE_NEW_INIT_ROUTINES: u8 = 0x40;
const FC_BIND_PRIMITIVE: u8 = 0x32;
const INTERPRETER_OPT_FLAGS2_NEW_CORRELATION_DESCRIPTOR: u8 = 1;
const INTERPRETER_OPT_FLAGS2_RANGE_ON_CONFORMANCE: u8 = 0x40;
const PARAM_ATTRIBUTES_MUST_SIZE: u16 = 0x1;
const PARAM_ATTRIBUTES_MUST_FREE: u16 = 0x2;
const PARAM_ATTRIBUTES_IS_IN: u16 = 0x8;
const PARAM_ATTRIBUTES_IS_OUT: u16 = 0x10;
const PARAM_ATTRIBUTES_IS_RETURN: u16 = 0x20;
const PARAM_ATTRIBUTES_IS_BASE_TYPE: u16 = 0x40;
const PARAM_ATTRIBUTES_IS_BY_VALUE: u16 = 0x80;
const PARAM_ATTRIBUTES_IS_SIMPLE_REF: u16 = 0x100;
// Following consts can be mixed to create 8 + 16 + 32 = 56 bytes.
const PARAM_ATTRIBUTES_SERVER_ALLOC_SIZE_8: u16 = 0x2000;
const PARAM_ATTRIBUTES_SERVER_ALLOC_SIZE_16: u16 = 0x4000;
const PARAM_ATTRIBUTES_SERVER_ALLOC_SIZE_32: u16 = 0x8000;
// Type format string constants
const FC_RP: u8 = 0x11; // Reference pointer
const FC_UP: u8 = 0x12; // Unique pointer
const FC_C_CSTRING: u8 = 0x22; // Conformant character string
const FC_PAD: u8 = 0x5c; // Padding
const FC_SIMPLE_POINTER: u8 = 0x8; // Simple pointer flag

// NDR64 Transfer Syntax
const RPC_TRANSFER_SYNTAX_NDR64_GUID: u128 = 0x71710533_beba_4937_8319_b5dbef9ccc36;

// NDR64 Format Codes (for base types)
const NDR64_FC_INT8: u8 = 0x10;
const NDR64_FC_INT16: u8 = 0x04;
const NDR64_FC_INT32: u8 = 0x05;
const NDR64_FC_INT64: u8 = 0x07;

// NDR64 Parameter Attributes
const NDR64_IS_IN: u16 = 0x0008;
const NDR64_IS_OUT: u16 = 0x0010;
const NDR64_IS_RETURN: u16 = 0x0020;
const NDR64_IS_BASE_TYPE: u16 = 0x0040;
const NDR64_IS_BY_VALUE: u16 = 0x0080;

#[derive(Default)]
struct InterfaceVersion {
    major: u16,
    minor: u16,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[repr(u8)]
enum BaseType {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    I64,
    U64,
}

impl BaseType {
    fn to_fc_value(&self) -> u8 {
        match self {
            BaseType::U8 => 1,
            BaseType::I8 => 2,
            BaseType::U16 => 6,
            BaseType::I16 => 7,
            BaseType::U32 => 8,
            BaseType::I32 => 9,
            BaseType::I64 => 11,
            BaseType::U64 => 11,
        }
    }

    fn to_ndr64_fc_value(&self) -> u8 {
        match self {
            BaseType::U8 | BaseType::I8 => NDR64_FC_INT8,
            BaseType::U16 | BaseType::I16 => NDR64_FC_INT16,
            BaseType::U32 | BaseType::I32 => NDR64_FC_INT32,
            BaseType::U64 | BaseType::I64 => NDR64_FC_INT64,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
enum Type {
    //Pointer(Box<Type>),
    String,
    Simple(BaseType),
}

impl TryFrom<SynType> for Type {
    type Error = syn::Error;

    fn try_from(value: syn::Type) -> Result<Self, Self::Error> {
        let SynType::Path(path) = &value else {
            return Err(syn::Error::new_spanned(
                value.to_token_stream(),
                "Only functions are allowed on this trait",
            ));
        };
        let ident = path.path.require_ident()?;
        // FIXME: for each enum variant?
        let res = if ident == "u8" {
            Self::Simple(BaseType::U8)
        } else if ident == "i8" {
            Self::Simple(BaseType::I8)
        } else if ident == "i16" {
            Self::Simple(BaseType::I16)
        } else if ident == "u16" {
            Self::Simple(BaseType::U16)
        } else if ident == "i32" {
            Self::Simple(BaseType::I32)
        } else if ident == "u32" {
            Self::Simple(BaseType::U32)
        } else if ident == "i64" {
            Self::Simple(BaseType::I64)
        } else if ident == "u64" {
            Self::Simple(BaseType::U64)
        } else {
            return Err(syn::Error::new_spanned(
                ident.to_token_stream(),
                "Unsupported type was used",
            ));
        };

        Ok(res)
    }
}

impl Type {
    fn to_rust_type(&self) -> proc_macro2::TokenStream {
        match self {
            Type::String => quote! { &str },
            Type::Simple(BaseType::U8) => quote! { u8 },
            Type::Simple(BaseType::I8) => quote! { i8 },
            Type::Simple(BaseType::U16) => quote! { u16 },
            Type::Simple(BaseType::I16) => quote! { i16 },
            Type::Simple(BaseType::U32) => quote! { u32 },
            Type::Simple(BaseType::I32) => quote! { i32 },
            Type::Simple(BaseType::U64) => quote! { u64 },
            Type::Simple(BaseType::I64) => quote! { i64 },
        }
    }

    fn rust_type_to_abi(&self, name: Ident) -> proc_macro2::TokenStream {
        match self {
            Type::String => quote! {
                std::mem::transmute_copy::<HSTRING, PCWSTR>(&HSTRING::from(#name))
            },
            // Simple types are passed as-is through the ABI
            Type::Simple(_) => quote! { #name },
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
struct Parameter {
    pub r#type: Type,
    pub name: String,
    pub is_in: bool,
    pub is_out: bool,
}

impl Parameter {
    /// Generates the [PARAM_ATTRIBUTES](https://learn.microsoft.com/en-us/windows/win32/rpc/parameter-descriptors#the-oif-parameter-descriptors)
    fn param_attributes(&self) -> u16 {
        let mut attributes = 0;
        if self.is_in {
            attributes |= PARAM_ATTRIBUTES_IS_IN;
        }
        if self.is_out {
            attributes |= PARAM_ATTRIBUTES_IS_OUT;
        }

        match self.r#type {
            Type::String => {
                attributes |= PARAM_ATTRIBUTES_MUST_SIZE
                    | PARAM_ATTRIBUTES_MUST_FREE
                    | PARAM_ATTRIBUTES_IS_SIMPLE_REF;
            }
            Type::Simple(_) => attributes |= PARAM_ATTRIBUTES_IS_BASE_TYPE,
        }

        attributes
    }

    fn ndr64_param_attributes(&self) -> u16 {
        let mut attributes = 0;
        if self.is_in {
            attributes |= NDR64_IS_IN;
        }
        if self.is_out {
            attributes |= NDR64_IS_OUT;
        }

        match self.r#type {
            Type::String => {
                // String parameters need special handling in NDR64
            }
            Type::Simple(_) => attributes |= NDR64_IS_BASE_TYPE | NDR64_IS_BY_VALUE,
        }

        attributes
    }
}

struct Method {
    pub return_type: Option<Type>,
    pub name: String,
    pub parameters: Vec<Parameter>,
}

#[derive(Default)]
struct Interface {
    pub name: String,
    pub uuid: GUID,
    pub version: InterfaceVersion,
    pub methods: Vec<Method>,
}

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

fn ndr_fc_long(value: u32) -> [u8; 4] {
    [
        (value & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        ((value >> 16) & 0xFF) as u8,
        ((value >> 24) & 0xFF) as u8,
    ]
}

fn ndr_fc_short(value: u16) -> [u8; 2] {
    [(value & 0xFF) as u8, ((value >> 8) & 0xFF) as u8]
}

fn generate_type_format_string(interface: &Interface) -> (Vec<u8>, HashMap<Parameter, u16>) {
    let mut type_format = vec![];
    let mut type_offsets = HashMap::new();

    // Start with padding short (always 0)
    type_format.extend_from_slice(&ndr_fc_short(0));

    // Collect all unique types that need descriptors
    let mut types_to_process = Vec::new();
    for method in &interface.methods {
        for param in &method.parameters {
            if !matches!(param.r#type, Type::Simple(_)) {
                if !type_offsets.contains_key(param) {
                    types_to_process.push(param.clone());
                }
            }
        }
    }

    // Generate type descriptors
    for param in types_to_process {
        let current_offset = type_format.len() as u16;
        type_offsets.insert(param.clone(), current_offset);

        match param.r#type {
            Type::String => {
                if param.is_in && !param.is_out {
                    // Simple pointer to conformant string (for [in] parameters)
                    // FC_RP [simple_pointer]
                    type_format.push(FC_RP);
                    type_format.push(FC_SIMPLE_POINTER);
                    // FC_C_CSTRING
                    type_format.push(FC_C_CSTRING);
                    type_format.push(FC_PAD);
                } else if param.is_out {
                    // Pointer to pointer to conformant string (for [out] parameters)
                    // This is more complex and needs multiple levels
                    // FC_RP [alloced_on_stack] [pointer_deref]
                    type_format.push(FC_RP);
                    type_format.push(0x14); // alloced_on_stack | pointer_deref
                    // Offset to the next pointer descriptor
                    type_format.extend_from_slice(&ndr_fc_short(2));

                    // FC_UP [simple_pointer]
                    type_format.push(FC_UP);
                    type_format.push(FC_SIMPLE_POINTER);
                    // FC_C_CSTRING
                    type_format.push(FC_C_CSTRING);
                    type_format.push(FC_PAD);
                }
            }
            Type::Simple(_) => {
                // Simple types don't need type descriptors
            }
        }
    }

    // End marker
    type_format.push(0);

    (type_format, type_offsets)
}

fn generate_ndr64_type_format(interface: &Interface) -> Vec<u8> {
    let mut type_format_bytes = vec![];
    let mut seen_types = std::collections::HashSet::new();

    // Type fragments must be contiguous in memory (not separately boxed)
    // For NDR64, even base types need type descriptors that can be pointed to
    // Collect all unique types and write them sequentially into one Vec<u8>

    for method in &interface.methods {
        // Process parameters
        for param in &method.parameters {
            if !seen_types.contains(&param.r#type) {
                seen_types.insert(param.r#type.clone());

                // Create type fragment based on param type
                match &param.r#type {
                    Type::String => {
                        // NDR64 string descriptor
                        // Simplified for initial implementation
                        type_format_bytes.push(0); // Placeholder
                    }
                    Type::Simple(bt) => {
                        // Base types need a single-byte descriptor
                        type_format_bytes.push(bt.to_ndr64_fc_value());
                    }
                }
            }
        }

        // Process return type
        if let Some(ref return_type) = method.return_type {
            if !seen_types.contains(return_type) {
                seen_types.insert(return_type.clone());

                match return_type {
                    Type::String => {
                        type_format_bytes.push(0); // Placeholder
                    }
                    Type::Simple(bt) => {
                        type_format_bytes.push(bt.to_ndr64_fc_value());
                    }
                }
            }
        }
    }

    type_format_bytes
}

// Returns proc header and procedure offsets
fn generate_proc_header(
    interface: &Interface,
    type_offsets: &HashMap<Parameter, u16>,
) -> (Vec<u8>, Vec<u16>) {
    let mut header = vec![];
    let mut proc_offsets: Vec<u16> = vec![];

    for (proc_index, proc) in interface.methods.iter().enumerate() {
        proc_offsets.push(header.len().try_into().unwrap());

        // Explicit handle
        header.push(0);
        // Oi_flags
        header.push(Oi_HAS_RPCFLAGS | Oi_USE_NEW_INIT_ROUTINES);
        // rpc_flags
        header.extend_from_slice(&ndr_fc_long(0));
        // proc_num
        header.extend_from_slice(&ndr_fc_short(proc_index.try_into().unwrap()));
        // Stack size - the total size of all parameters on the stack,
        // including any this pointer and/or return value
        header.extend_from_slice(&ndr_fc_short(0)); // TODO

        // Explicit handle
        // handle_type
        header.push(FC_BIND_PRIMITIVE);
        // IsPassByPointer
        header.push(0);
        // Offset from the beginning of the stack to the primitive handle.
        // We always pass it as the first parameter, so offset is 0
        header.extend_from_slice(&ndr_fc_short(0));
        // constant_client_buffer_size
        // This may be only a partial size, as the ClientMustSize flag triggers the sizing.
        header.extend_from_slice(&ndr_fc_short(0)); // TODO
        // constant_server_buffer_size
        // This may be only a partial size, as the ServerMustSize flag triggers the sizing
        header.extend_from_slice(&ndr_fc_short(0)); // TODO
        // INTERPRETER_OPT_FLAGS
        header.push(0x40); // has ext // TODO
        // Number of parameters
        header.push(proc.parameters.len().try_into().unwrap());

        // Extension section
        // extension_version (size of this section in bytes)
        header.push(10);
        // INTERPRETER_OPT_FLAGS2
        // FIXME: when do we set ServerCorrCheck and ClientCorrCheck?
        // https://learn.microsoft.com/en-us/windows/win32/rpc/the-header
        header.push(
            // FIXME: this is wrong when there are parameters?
            // INTERPRETER_OPT_FLAGS2_RANGE_ON_CONFORMANCE |
            INTERPRETER_OPT_FLAGS2_NEW_CORRELATION_DESCRIPTOR,
        );
        // ClientCorrHint - some cache hint for the client
        // FIXME: figure out
        header.extend_from_slice(&ndr_fc_short(0));
        // ServerCorrHint - some cache hint for the server
        // FIXME: figure out
        header.extend_from_slice(&ndr_fc_short(0));
        // Notify routine index, if one is used
        header.extend_from_slice(&ndr_fc_short(0));
        // FloatDoubleMask - relevant only for 64-bit. We'll ignore for now.
        #[cfg(all(windows, target_pointer_width = "64"))]
        header.extend_from_slice(&ndr_fc_short(0));

        // Parameters
        // The first parameter is the RPC handle, skip it.
        let mut param_stack_offset = std::mem::size_of::<usize>() as u16;
        for param in &proc.parameters {
            // PARAM_ATTRIBUTES
            header.extend_from_slice(&ndr_fc_short(param.param_attributes()));
            // stack_offset
            header.extend_from_slice(&ndr_fc_short(param_stack_offset));
            // type_offset OR base type value for simple types
            if let Type::Simple(base_type) = &param.r#type {
                header.extend_from_slice(&ndr_fc_short(base_type.to_fc_value() as u16));
            } else {
                header.extend_from_slice(&ndr_fc_short(*type_offsets.get(param).unwrap()));
            }

            // We only support parameters that fit in usize for now, so this will be enough.
            param_stack_offset += std::mem::size_of::<usize>() as u16;
        }

        // Let's only support basic types for now. We should generate some error for other types
        if let Some(Type::Simple(return_type)) = &proc.return_type {
            // PARAM_ATTRIBUTES
            header.extend_from_slice(&ndr_fc_short(
                PARAM_ATTRIBUTES_IS_OUT
                    | PARAM_ATTRIBUTES_IS_RETURN
                    | PARAM_ATTRIBUTES_IS_BASE_TYPE,
            ));
            // stack_offset
            header.extend_from_slice(&ndr_fc_short(param_stack_offset));
            // type_offset OR base type value for simple types
            header.extend_from_slice(&ndr_fc_short(return_type.to_fc_value() as u16)); // FIXME: put base type value
        }
    }

    // Zero marks the end of the header
    header.push(0);
    (header, proc_offsets)
}

// Helper to compute type offset in the ndr64_type_format buffer
fn compute_type_offset(interface: &Interface, target_type: &Type) -> usize {
    let mut offset = 0;
    let mut seen_types = std::collections::HashSet::new();

    for method in &interface.methods {
        for param in &method.parameters {
            if !seen_types.contains(&param.r#type) {
                if &param.r#type == target_type {
                    return offset;
                }
                seen_types.insert(param.r#type.clone());
                offset += 1; // Each type descriptor is 1 byte for base types
            }
        }

        if let Some(ref return_type) = method.return_type {
            if !seen_types.contains(return_type) {
                if return_type == target_type {
                    return offset;
                }
                seen_types.insert(return_type.clone());
                offset += 1;
            }
        }
    }

    0 // Not found
}

fn generate_ndr64_proc_buffer_code(interface: &Interface) -> proc_macro2::TokenStream {
    use quote::quote;

    let mut proc_descriptors = vec![];

    for method in interface.methods.iter() {
        let param_count = method.parameters.len();
        let has_return = method.return_type.is_some();
        let total_params = param_count + if has_return { 1 } else { 0 };
        let stack_size = (8 + (total_params * 8)) as u32;

        let mut flags = 0x01000040u32; // NDR64_HAS_EXT | explicit handle
        if has_return {
            flags |= 0x00080000; // HasReturn flag
        }

        // FIXME: this is only true for simple types
        let constant_client_buffer_size =
            (method.parameters.len() * std::mem::size_of::<usize>()) as u32;
        let constant_server_buffer_size = if has_return { 8u32 } else { 0u32 };

        // Generate proc format struct
        let proc_format = quote! {
            windows::Win32::System::Rpc::NDR64_PROC_FORMAT {
                Flags: #flags,
                StackSize: #stack_size,
                ConstantClientBufferSize: #constant_client_buffer_size,
                ConstantServerBufferSize: #constant_server_buffer_size,
                RpcFlags: 0,
                FloatDoubleMask: 0,
                NumberOfParams: #total_params as u16,
                ExtensionSize: 8,
            }
        };

        // Generate bind extension
        let bind_extension = quote! {
            windows::Win32::System::Rpc::NDR64_BIND_AND_NOTIFY_EXTENSION {
                Binding: windows::Win32::System::Rpc::NDR64_BIND_CONTEXT {
                    HandleType: 0x72, // FC64_BIND_PRIMITIVE
                    Flags: 0,
                    StackOffset: 0,
                    RoutineIndex: 0,
                    Ordinal: 0,
                },
                NotifyIndex: 0,
            }
        };

        // Generate parameter descriptors
        let mut param_descriptors = vec![];
        let mut stack_offset = 8u32;

        for param in &method.parameters {
            let type_offset = compute_type_offset(interface, &param.r#type);
            let attributes = param.ndr64_param_attributes();

            param_descriptors.push(quote! {
                windows::Win32::System::Rpc::NDR64_PARAM_FORMAT {
                    Type: unsafe { ndr64_type_format.as_ptr().add(#type_offset) as *mut core::ffi::c_void },
                    Attributes: windows::Win32::System::Rpc::NDR64_PARAM_FLAGS {
                        _bitfield: #attributes,
                    },
                    Reserved: 0,
                    StackOffset: #stack_offset,
                }
            });

            stack_offset += 8;
        }

        // Generate return value descriptor if present
        if let Some(ref return_type) = method.return_type {
            let type_offset = compute_type_offset(interface, return_type);

            param_descriptors.push(quote! {
                windows::Win32::System::Rpc::NDR64_PARAM_FORMAT {
                    Type: unsafe { ndr64_type_format.as_ptr().add(#type_offset) as *mut core::ffi::c_void },
                    Attributes: windows::Win32::System::Rpc::NDR64_PARAM_FLAGS {
                        // FIXME: properly compute according to type
                        _bitfield: 0x00f0, // IS_OUT | IS_RETURN | IS_BASE_TYPE | IS_BY_VALUE
                    },
                    Reserved: 0,
                    StackOffset: #stack_offset,
                }
            });
        }

        proc_descriptors.push(quote! {
            {
                let proc_format = #proc_format;
                proc_buffer.extend_from_slice(unsafe {
                    std::slice::from_raw_parts(
                        &proc_format as *const _ as *const u8,
                        std::mem::size_of::<windows::Win32::System::Rpc::NDR64_PROC_FORMAT>(),
                    )
                });

                let bind_extension = #bind_extension;
                proc_buffer.extend_from_slice(unsafe {
                    std::slice::from_raw_parts(
                        &bind_extension as *const _ as *const u8,
                        std::mem::size_of::<windows::Win32::System::Rpc::NDR64_BIND_AND_NOTIFY_EXTENSION>(),
                    )
                });

                #(
                    {
                        let param_format = #param_descriptors;
                        proc_buffer.extend_from_slice(unsafe {
                            std::slice::from_raw_parts(
                                &param_format as *const _ as *const u8,
                                std::mem::size_of::<windows::Win32::System::Rpc::NDR64_PARAM_FORMAT>(),
                            )
                        });
                    }
                )*
            }
        });
    }

    quote! {
        {
            let mut proc_buffer: Vec<u8> = Vec::new();
            let mut proc_table_offsets: Vec<usize> = Vec::new();

            #(
                proc_table_offsets.push(proc_buffer.len());
                #proc_descriptors
            )*

            (proc_buffer, proc_table_offsets)
        }
    }
}

fn compile_client(interface: Interface) -> proc_macro2::TokenStream {
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
                return_type: Some(crate::Type::Simple(BaseType::U64)),
                name: "NoParams".to_string(),
                parameters: vec![],
            },
            Method {
                return_type: Some(crate::Type::Simple(BaseType::I32)),
                name: "SingleParamReturn".to_string(),
                parameters: vec![Parameter {
                    r#type: crate::Type::Simple(BaseType::I32),
                    name: "foo".to_owned(),
                    is_in: true,
                    is_out: false,
                }],
            },
            Method {
                return_type: Some(crate::Type::Simple(BaseType::I32)),
                name: "Sum".to_string(),
                parameters: vec![
                    Parameter {
                        r#type: crate::Type::Simple(BaseType::I32),
                        name: "a".to_owned(),
                        is_in: true,
                        is_out: false,
                    },
                    Parameter {
                        r#type: crate::Type::Simple(BaseType::I32),
                        name: "b".to_owned(),
                        is_in: true,
                        is_out: false,
                    },
                ],
            },
        ],
        ..Default::default()
    })
    .into()
}
