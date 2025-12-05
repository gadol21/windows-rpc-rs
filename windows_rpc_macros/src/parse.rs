use syn::{Ident, LitFloat, LitInt, Token, parse::Parse};

use crate::types::InterfaceVersion;

/// Parsed attributes for the rpc_interface macro
pub struct InterfaceAttributes {
    pub guid: u128,
    pub version: InterfaceVersion,
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
                        format!("Unknown attribute: {other}"),
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
