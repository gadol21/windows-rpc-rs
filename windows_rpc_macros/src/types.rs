use quote::{ToTokens, quote};
use syn::Type as SynType;
use windows::core::GUID;

use crate::constants::*;

#[derive(Default, Clone)]
pub struct InterfaceVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[repr(u8)]
pub enum BaseType {
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
    pub fn to_fc_value(&self) -> u8 {
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

    pub fn to_ndr64_fc_value(&self) -> u8 {
        match self {
            BaseType::U8 | BaseType::I8 => NDR64_FC_INT8,
            BaseType::U16 | BaseType::I16 => NDR64_FC_INT16,
            BaseType::U32 | BaseType::I32 => NDR64_FC_INT32,
            BaseType::U64 | BaseType::I64 => NDR64_FC_INT64,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum Type {
    //Pointer(Box<Type>),
    String,
    Simple(BaseType),
}

impl TryFrom<SynType> for Type {
    type Error = syn::Error;

    fn try_from(value: syn::Type) -> Result<Self, Self::Error> {
        // Handle &str
        if let SynType::Reference(ref_type) = &value {
            if let SynType::Path(path) = &*ref_type.elem {
                if path.path.is_ident("str") {
                    return Ok(Self::String);
                }
            }
        }

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
    pub fn to_rust_type(&self) -> proc_macro2::TokenStream {
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

    pub fn rust_type_to_abi(&self, name: syn::Ident) -> proc_macro2::TokenStream {
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
pub struct Parameter {
    pub r#type: Type,
    pub name: String,
    pub is_in: bool,
    pub is_out: bool,
}

impl Parameter {
    /// Generates the [PARAM_ATTRIBUTES](https://learn.microsoft.com/en-us/windows/win32/rpc/parameter-descriptors#the-oif-parameter-descriptors)
    pub fn param_attributes(&self) -> u16 {
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

    pub fn ndr64_param_attributes(&self) -> u16 {
        let mut attributes = 0;
        if self.is_in {
            attributes |= NDR64_IS_IN;
        }
        if self.is_out {
            attributes |= NDR64_IS_OUT;
        }

        match self.r#type {
            Type::String => {
                // String parameters need MustSize, MustFree, and SimpleRef flags
                attributes |= NDR64_MUST_SIZE | NDR64_MUST_FREE | NDR64_IS_SIMPLE_REF;
            }
            Type::Simple(_) => attributes |= NDR64_IS_BASE_TYPE | NDR64_IS_BY_VALUE,
        }

        attributes
    }
}

#[derive(Clone)]
pub struct Method {
    pub return_type: Option<Type>,
    pub name: String,
    pub parameters: Vec<Parameter>,
}

#[derive(Default, Clone)]
pub struct Interface {
    pub name: String,
    pub uuid: GUID,
    pub version: InterfaceVersion,
    pub methods: Vec<Method>,
}

impl Interface {
    /// Returns an iterator over all unique types in the interface (parameters and return types)
    pub fn unique_types(&self) -> impl Iterator<Item = &Type> {
        let mut seen = std::collections::HashSet::new();
        self.methods
            .iter()
            .flat_map(|m| {
                m.parameters
                    .iter()
                    .map(|p| &p.r#type)
                    .chain(m.return_type.iter())
            })
            .filter(move |t| seen.insert((*t).clone()))
            .collect::<Vec<_>>()
            .into_iter()
    }
}
