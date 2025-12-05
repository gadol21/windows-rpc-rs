use quote::quote;

use crate::constants::NDR64_FC_CONF_WCHAR_STRING;
use crate::types::{Interface, Type};

pub fn generate_ndr64_type_format(interface: &Interface) -> Vec<u8> {
    // Type fragments must be contiguous in memory (not separately boxed)
    // For NDR64, even base types need type descriptors that can be pointed to
    // Collect all unique types and write them sequentially into one Vec<u8>
    let mut type_format = vec![];
    for t in interface.unique_types() {
        match t {
            Type::String => {
                // NDR64_CONFORMANT_STRING_FORMAT (4 bytes)
                type_format.push(NDR64_FC_CONF_WCHAR_STRING); // 0x64
                type_format.push(0); // flags byte
                type_format.extend_from_slice(&2u16.to_le_bytes()); // element size = 2 for wchar_t
            }
            Type::Simple(bt) => {
                type_format.push(bt.to_ndr64_fc_value());
            }
        }
    }
    type_format
}

// Helper to compute type offset in the ndr64_type_format buffer
// Note: Strings take 4 bytes, simple types take 1 byte
pub fn compute_type_offset(interface: &Interface, target_type: &Type) -> usize {
    let mut offset = 0;
    for t in interface.unique_types() {
        if t == target_type {
            return offset;
        }
        // Strings are 4 bytes (format code + flags + element size u16)
        // Simple types are 1 byte
        offset += match t {
            Type::String => 4,
            Type::Simple(_) => 1,
        };
    }
    0 // Not found
}

pub fn generate_ndr64_proc_buffer_code(interface: &Interface) -> proc_macro2::TokenStream {
    let mut proc_descriptors = vec![];

    for method in interface.methods.iter() {
        let param_count = method.parameters.len();
        let has_return = method.return_type.is_some();
        let total_params = param_count + if has_return { 1 } else { 0 };
        let stack_size = (8 + (total_params * 8)) as u32;

        let has_string_param = method
            .parameters
            .iter()
            .any(|p| matches!(p.r#type, Type::String));

        let mut flags = 0x01000040u32; // NDR64_HAS_EXT | explicit handle
        if has_return {
            flags |= 0x00080000; // HasReturn flag
        }
        if has_string_param {
            flags |= crate::constants::NDR64_PROC_CLIENT_MUST_SIZE; // 0x00040000
        }

        // For string params, sizing is required so buffer size is 0
        // For simple types only, we can compute the constant buffer size
        let constant_client_buffer_size = if has_string_param {
            0u32
        } else {
            (method.parameters.len() * std::mem::size_of::<usize>()) as u32
        };

        // Add one usize for binding handle.
        let constant_server_buffer_size =
            std::mem::size_of::<usize>() as u32 + if has_return { 8u32 } else { 0u32 };

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
