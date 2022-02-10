use crate::models::{
    Callback, Constant, Enumeration, Error, Flags, Function, OpaqueType, Structure, TypeAlias,
};

use quote::__private::{LexError, Literal, TokenStream};
use quote::quote;
use std::collections::HashMap;
use std::num::ParseIntError;
use std::os::raw::{c_uint, c_ulonglong};
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct Api {
    pub opaque_types: Vec<OpaqueType>,
    pub constants: Vec<Constant>,
    pub flags: Vec<Flags>,
    pub enumerations: Vec<Enumeration>,
    pub structures: Vec<Structure>,
    pub callbacks: Vec<Callback>,
    pub type_aliases: Vec<TypeAlias>,
    pub functions: HashMap<String, Vec<Function>>,
}

impl From<rustfmt_wrapper::Error> for Error {
    fn from(error: rustfmt_wrapper::Error) -> Self {
        Error::Fmt(format!("{:?}", error))
    }
}

impl From<ParseIntError> for Error {
    fn from(error: ParseIntError) -> Self {
        Error::ParseInt(error.to_string())
    }
}

impl From<LexError> for Error {
    fn from(error: LexError) -> Self {
        Error::LexError(error.to_string())
    }
}

pub fn generate_opaque_type_code(value: &OpaqueType) -> TokenStream {
    let name = format_ident!("{}", value.name);

    quote! {
        #[repr(C)]
        #[derive(Debug, Copy, Clone)]
        pub struct #name {
            _unused: [u8; 0]
        }
    }
}

pub fn generate_constant_code(constant: &Constant) -> Result<TokenStream, Error> {
    let name = format_ident!("{}", &constant.name);
    let value = &constant.value;

    let tokens = if value.len() == "0xFFFFFFFFFFFFFFFF".len() && value.starts_with("0x") {
        let value = TokenStream::from_str(value)?;
        quote! {
            pub const #name: raw::c_ulonglong = #value;
        }
    } else {
        let value = Literal::u32_unsuffixed(value.parse()?);
        quote! {
            pub const #name: raw::c_uint = #value;
        }
    };

    Ok(tokens)
}

pub fn generate_api_code(api: Api) -> Result<TokenStream, Error> {
    let opaque_types: Vec<TokenStream> = api
        .opaque_types
        .iter()
        .map(generate_opaque_type_code)
        .collect();

    let mut constants = vec![];
    for constant in &api.constants {
        constants.push(generate_constant_code(constant)?);
    }

    Ok(quote! {
        #![allow(non_camel_case_types)]
        use std::os::raw;

        #(#opaque_types)*
        #(#constants)*
    })
}

pub fn generate_api(api: Api) -> Result<String, Error> {
    let code = generate_api_code(api)?;
    rustfmt_wrapper::rustfmt(code).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use crate::ffi::{generate_api, Api};
    use crate::models::{Constant, OpaqueType};
    use quote::__private::TokenStream;

    fn format(code: TokenStream) -> String {
        rustfmt_wrapper::rustfmt(code).unwrap()
    }

    #[test]
    fn test_should_generate_int_constant() {
        let mut api = Api::default();
        api.constants.push(Constant {
            name: "FMOD_MAX_CHANNEL_WIDTH".into(),
            value: "32".into(),
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            use std::os::raw;

            pub const FMOD_MAX_CHANNEL_WIDTH: raw::c_uint = 32;
        };
        assert_eq!(generate_api(api), Ok(format(code)))
    }

    #[test]
    fn test_should_generate_hex_constant() {
        let mut api = Api::default();
        api.constants.push(Constant {
            name: "FMOD_PORT_INDEX_NONE".into(),
            value: "0xFFFFFFFFFFFFFFFF".into(),
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            use std::os::raw;

            pub const FMOD_PORT_INDEX_NONE: raw::c_ulonglong = 0xFFFFFFFFFFFFFFFF;
        };
        assert_eq!(generate_api(api), Ok(format(code)))
    }

    #[test]
    fn test_should_generate_opaque_type() {
        let mut api = Api::default();
        api.opaque_types.push(OpaqueType {
            name: "FMOD_SOUND".into(),
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            use std::os::raw;

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_SOUND {
                _unused: [u8; 0]
            }
        };
        assert_eq!(generate_api(api), Ok(format(code)))
    }

    #[test]
    fn test_should_generate_multiple_opaque_types() {
        let mut api = Api::default();
        api.opaque_types.push(OpaqueType {
            name: "FMOD_SOUND".into(),
        });
        api.opaque_types.push(OpaqueType {
            name: "FMOD_CHANNELCONTROL".into(),
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            use std::os::raw;

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_SOUND {
                _unused: [u8; 0]
            }

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_CHANNELCONTROL {
                _unused: [u8; 0]
            }
        };
        assert_eq!(generate_api(api), Ok(format(code)))
    }
}
