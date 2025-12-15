use std::collections::HashMap;

use crate::constants::*;
use crate::types::{Interface, Parameter, Type};

pub fn ndr_fc_long(value: u32) -> [u8; 4] {
    [
        (value & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        ((value >> 16) & 0xFF) as u8,
        ((value >> 24) & 0xFF) as u8,
    ]
}

pub fn ndr_fc_short(value: u16) -> [u8; 2] {
    [(value & 0xFF) as u8, ((value >> 8) & 0xFF) as u8]
}

/// Key for type offset lookup - can be a parameter or a return type marker
#[derive(PartialEq, Eq, Hash, Clone)]
pub enum TypeKey {
    Parameter(Parameter),
    ReturnString, // Out string for return value
}

pub fn generate_type_format_string(interface: &Interface) -> (Vec<u8>, HashMap<TypeKey, u16>) {
    let mut type_format = vec![];
    let mut type_offsets = HashMap::new();

    // Start with padding short (always 0)
    type_format.extend_from_slice(&ndr_fc_short(0));

    // Collect all unique types that need descriptors
    let mut types_to_process = Vec::new();
    for method in &interface.methods {
        for param in &method.parameters {
            if !matches!(param.r#type, Type::Simple(_))
                && !type_offsets.contains_key(&TypeKey::Parameter(param.clone()))
            {
                types_to_process.push(TypeKey::Parameter(param.clone()));
            }
        }
        // Check if method has a string return type
        if let Some(Type::String) = &method.return_type
            && !type_offsets.contains_key(&TypeKey::ReturnString)
        {
            types_to_process.push(TypeKey::ReturnString);
        }
    }

    // Generate type descriptors
    for type_key in types_to_process {
        let current_offset = type_format.len() as u16;
        type_offsets.insert(type_key.clone(), current_offset);

        match &type_key {
            TypeKey::Parameter(param) => match param.r#type {
                Type::String => {
                    if param.is_in && !param.is_out {
                        // Simple pointer to conformant string (for [in] parameters)
                        // FC_RP [simple_pointer]
                        type_format.push(FC_RP);
                        type_format.push(FC_SIMPLE_POINTER);
                        // FC_C_WSTRING (unicode wide string)
                        type_format.push(FC_C_WSTRING);
                        type_format.push(FC_PAD);
                    } else if param.is_out {
                        // Pointer to pointer to conformant string (for [out] parameters)
                        // FC_RP [alloced_on_stack] [pointer_deref]
                        type_format.push(FC_RP);
                        type_format.push(0x14); // alloced_on_stack | pointer_deref
                        // Offset to the next pointer descriptor
                        type_format.extend_from_slice(&ndr_fc_short(2));

                        // FC_UP [simple_pointer]
                        type_format.push(FC_UP);
                        type_format.push(FC_SIMPLE_POINTER);
                        // FC_C_WSTRING (unicode wide string)
                        type_format.push(FC_C_WSTRING);
                        type_format.push(FC_PAD);
                    }
                }
                Type::Simple(_) => {
                    // Simple types don't need type descriptors
                }
            },
            TypeKey::ReturnString => {
                // Out string return value: wchar_t**
                // FC_RP [alloced_on_stack] [pointer_deref]
                type_format.push(FC_RP);
                type_format.push(0x14); // alloced_on_stack | pointer_deref
                // Offset to the next pointer descriptor
                type_format.extend_from_slice(&ndr_fc_short(2));

                // FC_UP [simple_pointer]
                type_format.push(FC_UP);
                type_format.push(FC_SIMPLE_POINTER);
                // FC_C_WSTRING (unicode wide string)
                type_format.push(FC_C_WSTRING);
                type_format.push(FC_PAD);
            }
        }
    }

    // End marker
    type_format.push(0);

    (type_format, type_offsets)
}

// Returns proc header and procedure offsets
pub fn generate_proc_header(
    interface: &Interface,
    type_offsets: &HashMap<TypeKey, u16>,
) -> (Vec<u8>, Vec<u16>) {
    let mut header = vec![];
    let mut proc_offsets: Vec<u16> = vec![];

    for (proc_index, proc) in interface.methods.iter().enumerate() {
        proc_offsets.push(header.len().try_into().unwrap());

        // Calculate stack size:
        // - 8 bytes for binding handle (first implicit param)
        // - 8 bytes per parameter
        // - 8 bytes for return value (if simple type) or out string pointer (if string)
        let param_count = proc.parameters.len();
        let has_return_on_stack = proc.return_type.is_some();
        let stack_size = (8 + (param_count * 8) + if has_return_on_stack { 8 } else { 0 }) as u16;

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
        header.extend_from_slice(&ndr_fc_short(stack_size));

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
        // INTERPRETER_OPT_FLAGS (OI2 flags)
        let has_string_param = proc
            .parameters
            .iter()
            .any(|p| matches!(p.r#type, Type::String));
        let has_string_return = matches!(proc.return_type, Some(Type::String));
        let has_return = proc.return_type.is_some();
        // Count params including out string return value (which becomes an out param)
        let param_count = proc.parameters.len() + if has_string_return { 1 } else { 0 };
        let oi2_flags = 0x40 // has ext
            | if has_return && !has_string_return { 0x04 } else { 0 } // has return (only for simple types)
            | if has_string_param { OI2_CLIENT_MUST_SIZE } else { 0 } // client must size
            | if has_string_return { OI2_SERVER_MUST_SIZE } else { 0 }; // server must size
        header.push(oi2_flags);
        // Number of parameters (includes out string if returning string)
        header.push(param_count.try_into().unwrap());

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
                header.extend_from_slice(&ndr_fc_short(
                    *type_offsets
                        .get(&TypeKey::Parameter(param.clone()))
                        .unwrap(),
                ));
            }

            // We only support parameters that fit in usize for now, so this will be enough.
            param_stack_offset += std::mem::size_of::<usize>() as u16;
        }

        // Handle return type
        match &proc.return_type {
            Some(Type::Simple(return_type)) => {
                // Simple type return value
                // PARAM_ATTRIBUTES
                header.extend_from_slice(&ndr_fc_short(
                    PARAM_ATTRIBUTES_IS_OUT
                        | PARAM_ATTRIBUTES_IS_RETURN
                        | PARAM_ATTRIBUTES_IS_BASE_TYPE,
                ));
                // stack_offset
                header.extend_from_slice(&ndr_fc_short(param_stack_offset));
                // type_offset OR base type value for simple types
                header.extend_from_slice(&ndr_fc_short(return_type.to_fc_value() as u16));
            }
            Some(Type::String) => {
                // String return value becomes an out parameter (wchar_t**)
                // PARAM_ATTRIBUTES: 0x2013 = MUST_SIZE | MUST_FREE | IS_OUT | SERVER_ALLOC_SIZE_8
                header.extend_from_slice(&ndr_fc_short(
                    PARAM_ATTRIBUTES_MUST_SIZE
                        | PARAM_ATTRIBUTES_MUST_FREE
                        | PARAM_ATTRIBUTES_IS_OUT
                        | PARAM_ATTRIBUTES_SERVER_ALLOC_SIZE_8,
                ));
                // stack_offset
                header.extend_from_slice(&ndr_fc_short(param_stack_offset));
                // type_offset
                header.extend_from_slice(&ndr_fc_short(
                    *type_offsets.get(&TypeKey::ReturnString).unwrap(),
                ));
            }
            None => {}
        }
    }

    // Zero marks the end of the header
    header.push(0);
    (header, proc_offsets)
}
