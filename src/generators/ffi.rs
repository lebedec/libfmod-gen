use crate::models::{
    Callback, Constant, Enumeration, Error, Flags, Function, OpaqueType, Structure, TypeAlias,
};

use quote::__private::TokenStream;
use quote::quote;
use std::collections::HashMap;

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

pub fn generate_api_code(api: Api) -> TokenStream {
    let opaque_types: Vec<TokenStream> = api
        .opaque_types
        .iter()
        .map(generate_opaque_type_code)
        .collect();

    quote! {
        #![allow(non_camel_case_types)]

        #(#opaque_types)*
    }
}

pub fn generate_api(api: Api) -> Result<String, Error> {
    let code = generate_api_code(api);
    rustfmt_wrapper::rustfmt(code).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use crate::ffi::{generate_api, Api};
    use crate::models::OpaqueType;
    use quote::__private::TokenStream;

    fn format(code: TokenStream) -> String {
        rustfmt_wrapper::rustfmt(code).unwrap()
    }

    #[test]
    fn test_should_generate_opaque_type() {
        let mut api = Api::default();
        api.opaque_types.push(OpaqueType {
            name: "FMOD_SOUND".into(),
        });
        let code = quote! {
            #![allow(non_camel_case_types)]

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
