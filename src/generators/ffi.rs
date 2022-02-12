use crate::models::{
    Argument, Callback, Constant, Enumeration, Error, Flags, Function, OpaqueType, Pointer,
    Structure, Type, TypeAlias,
};

use crate::models::Type::FundamentalType;
use quote::__private::{Ident, LexError, Literal, TokenStream};
use quote::quote;
use std::collections::HashMap;
use std::num::ParseIntError;
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
            pub const #name: c_ulonglong = #value;
        }
    } else if value.len() == "0xaaaabbcc".len() && value.starts_with("0x") {
        let value = TokenStream::from_str(value)?;
        quote! {
            pub const #name: c_uint = #value;
        }
    } else {
        let value = Literal::u32_unsuffixed(value.parse()?);
        quote! {
            pub const #name: c_uint = #value;
        }
    };

    Ok(tokens)
}

pub fn map_type(c_type: &Type) -> Ident {
    let name = match c_type {
        FundamentalType(name) => match &name[..] {
            "char" => "c_char",
            "unsigned char" => "c_uchar",
            "signed char" => "c_char",
            "int" => "c_int",
            "unsigned int" => "c_unit",
            "short" => "c_short",
            "unsigned short" => "c_ushort",
            "long long" => "c_longlong",
            "long" => "c_long",
            "unsigned long long" => "c_ulonglong",
            "unsigned long" => "c_ulong",
            "float" => "c_float",
            _ => name,
        },
        Type::UserType(name) => name,
    };
    format_ident!("{}", name)
}

pub fn map_type2(
    c_type: &Type,
    as_const: &Option<String>,
    pointer: &Option<Pointer>,
) -> TokenStream {
    let name = match c_type {
        FundamentalType(name) => match &name[..] {
            "char" => "c_char",
            "unsigned char" => "c_uchar",
            "signed char" => "c_char",
            "int" => "c_int",
            "unsigned int" => "c_uint",
            "short" => "c_short",
            "unsigned short" => "c_ushort",
            "long long" => "c_longlong",
            "long" => "c_long",
            "unsigned long long" => "c_ulonglong",
            "unsigned long" => "c_ulong",
            "float" => "c_float",
            "void" => "c_void",
            "void*" => "c_void",
            _ => name,
        },
        Type::UserType(name) => name,
    };
    let name = format_ident!("{}", name);
    match (as_const, pointer) {
        (None, None) => quote! { #name },
        (None, Some(Pointer::NormalPointer(_))) => quote! { *mut #name },
        (None, Some(Pointer::DoublePointer(_))) => quote! { *mut *mut #name },
        (Some(_), Some(Pointer::NormalPointer(_))) => quote! { *const #name },
        (Some(_), Some(Pointer::DoublePointer(_))) => quote! { *const *const #name },
        (Some(_), None) => quote! { #name },
    }
}

pub fn generate_type_alias_code(type_alias: &TypeAlias) -> TokenStream {
    let name = format_ident!("{}", type_alias.name);
    let base = map_type(&type_alias.base_type);

    quote! {
        pub type #name = #base;
    }
}

pub fn generate_enumeration_code(enumeration: &Enumeration) -> Result<TokenStream, Error> {
    let name = format_ident!("{}", enumeration.name);
    let mut value: i32 = -1;
    let mut enumerators = vec![];
    for enumerator in &enumeration.enumerators {
        let label = format_ident!("{}", &enumerator.name);
        let value = match &enumerator.value {
            None => {
                value += 1;
                value
            }
            Some(repr) => {
                value = repr.parse()?;
                value
            }
        };
        let literal = Literal::i32_unsuffixed(value);
        enumerators.push(quote! {
            pub const #label: #name = #literal;
        });
    }
    Ok(quote! {
        pub type #name = c_int;
        #(#enumerators)*
    })
}

pub fn format_rust_ident(name: &String) -> Ident {
    let keywords = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
        "where", "while", "async", "await", "dyn", "try", "abstract", "become", "box", "do",
        "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield",
    ];
    if keywords.contains(&&*name.to_lowercase()) {
        format_ident!("{}_", name)
    } else {
        format_ident!("{}", name)
    }
}

pub fn generate_argument_code(argument: &Argument) -> TokenStream {
    let name = format_rust_ident(&argument.name);
    let argument_type = map_type2(
        &argument.argument_type,
        &argument.as_const,
        &argument.pointer,
    );
    quote! {
        #name: #argument_type
    }
}

pub fn generate_callback_code(callback: &Callback) -> TokenStream {
    let name = format_ident!("{}", callback.name);
    let arguments: Vec<TokenStream> = callback
        .arguments
        .iter()
        .map(generate_argument_code)
        .collect();

    let varargs = if callback.varargs.is_some() {
        Some(quote! {, ...})
    } else {
        None
    };

    if &callback.return_type == &FundamentalType("void".into()) && callback.pointer.is_none() {
        quote! {
            pub type #name = Option<unsafe extern "C" fn(#(#arguments),* #varargs)>;
        }
    } else {
        let return_type = map_type2(&callback.return_type, &None, &callback.pointer);
        quote! {
            pub type #name = Option<unsafe extern "C" fn(#(#arguments),* #varargs) -> #return_type>;
        }
    }
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

    let type_aliases: Vec<TokenStream> = api
        .type_aliases
        .iter()
        .map(generate_type_alias_code)
        .collect();

    let mut enumerations = vec![];
    for enumeration in &api.enumerations {
        enumerations.push(generate_enumeration_code(enumeration)?);
    }

    let callbacks: Vec<TokenStream> = api.callbacks.iter().map(generate_callback_code).collect();

    Ok(quote! {
        #![allow(non_camel_case_types)]
        #![allow(non_snake_case)]
        use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

        #(#opaque_types)*
        #(#type_aliases)*
        #(#constants)*
        #(#enumerations)*
        #(#callbacks)*
    })
}

pub fn generate_api(api: Api) -> Result<String, Error> {
    let code = generate_api_code(api)?;
    rustfmt_wrapper::rustfmt(code).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use crate::ffi::{generate_api, Api};
    use crate::models::Type::{FundamentalType, UserType};
    use crate::models::{
        Argument, Callback, Constant, Enumeration, Enumerator, OpaqueType, Pointer, TypeAlias,
    };
    use quote::__private::TokenStream;

    fn format(code: TokenStream) -> String {
        rustfmt_wrapper::rustfmt(code).unwrap()
    }

    fn normal() -> Option<Pointer> {
        Some(Pointer::NormalPointer("*".into()))
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
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

            pub const FMOD_MAX_CHANNEL_WIDTH: c_uint = 32;
        };
        assert_eq!(generate_api(api), Ok(format(code)))
    }

    #[test]
    fn test_should_generate_hex_long_constant() {
        let mut api = Api::default();
        api.constants.push(Constant {
            name: "FMOD_PORT_INDEX_NONE".into(),
            value: "0xFFFFFFFFFFFFFFFF".into(),
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

            pub const FMOD_PORT_INDEX_NONE: c_ulonglong = 0xFFFFFFFFFFFFFFFF;
        };
        assert_eq!(generate_api(api), Ok(format(code)))
    }

    #[test]
    fn test_should_generate_hex_int_constant() {
        let mut api = Api::default();
        api.constants.push(Constant {
            name: "FMOD_VERSION".into(),
            value: "0x00020203".into(),
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

            pub const FMOD_VERSION: c_uint = 0x00020203;
        };
        assert_eq!(generate_api(api), Ok(format(code)))
    }

    #[test]
    fn test_should_generate_type_alias() {
        let mut api = Api::default();
        api.type_aliases.push(TypeAlias {
            base_type: FundamentalType("unsigned long long".into()),
            name: "FMOD_PORT_INDEX".into(),
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

            pub type FMOD_PORT_INDEX = c_ulonglong;
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
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

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
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

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

    #[test]
    fn test_should_generate_enumeration_with_negative_value() {
        let mut api = Api::default();
        api.enumerations.push(Enumeration {
            name: "FMOD_CHANNELCONTROL_DSP_INDEX".into(),
            enumerators: vec![
                Enumerator {
                    name: "FMOD_CHANNELCONTROL_DSP_HEAD".into(),
                    value: Some("-1".into()),
                },
                Enumerator {
                    name: "FMOD_CHANNELCONTROL_DSP_FADER".into(),
                    value: Some("-2".into()),
                },
                Enumerator {
                    name: "FMOD_CHANNELCONTROL_DSP_FORCEINT".into(),
                    value: Some("65536".into()),
                },
            ],
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

            pub type FMOD_CHANNELCONTROL_DSP_INDEX = c_int;
            pub const FMOD_CHANNELCONTROL_DSP_HEAD: FMOD_CHANNELCONTROL_DSP_INDEX = -1;
            pub const FMOD_CHANNELCONTROL_DSP_FADER: FMOD_CHANNELCONTROL_DSP_INDEX = -2;
            pub const FMOD_CHANNELCONTROL_DSP_FORCEINT: FMOD_CHANNELCONTROL_DSP_INDEX = 65536;
        };
        assert_eq!(generate_api(api), Ok(format(code)))
    }

    #[test]
    fn test_should_generate_enumeration() {
        let mut api = Api::default();
        api.enumerations.push(Enumeration {
            name: "FMOD_PLUGINTYPE".into(),
            enumerators: vec![
                Enumerator {
                    name: "FMOD_PLUGINTYPE_OUTPUT".into(),
                    value: None,
                },
                Enumerator {
                    name: "FMOD_PLUGINTYPE_CODEC".into(),
                    value: None,
                },
            ],
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

            pub type FMOD_PLUGINTYPE = c_int;
            pub const FMOD_PLUGINTYPE_OUTPUT: FMOD_PLUGINTYPE = 0;
            pub const FMOD_PLUGINTYPE_CODEC: FMOD_PLUGINTYPE = 1;
        };
        assert_eq!(generate_api(api), Ok(format(code)))
    }

    #[test]
    fn test_should_generate_enumeration_with_start_values() {
        let mut api = Api::default();
        api.enumerations.push(Enumeration {
            name: "FMOD_SPEAKER".into(),
            enumerators: vec![
                Enumerator {
                    name: "FMOD_SPEAKER_NONE".into(),
                    value: Some("-1".into()),
                },
                Enumerator {
                    name: "FMOD_SPEAKER_FRONT_LEFT".into(),
                    value: Some("0".into()),
                },
                Enumerator {
                    name: "FMOD_SPEAKER_FRONT_RIGHT".into(),
                    value: None,
                },
                Enumerator {
                    name: "FMOD_SPEAKER_FRONT_CENTER".into(),
                    value: None,
                },
                Enumerator {
                    name: "FMOD_SPEAKER_FORCEINT".into(),
                    value: Some("65536".into()),
                },
            ],
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

            pub type FMOD_SPEAKER = c_int;
            pub const FMOD_SPEAKER_NONE: FMOD_SPEAKER = -1;
            pub const FMOD_SPEAKER_FRONT_LEFT: FMOD_SPEAKER = 0;
            pub const FMOD_SPEAKER_FRONT_RIGHT: FMOD_SPEAKER = 1;
            pub const FMOD_SPEAKER_FRONT_CENTER: FMOD_SPEAKER = 2;
            pub const FMOD_SPEAKER_FORCEINT: FMOD_SPEAKER = 65536;
        };
        assert_eq!(generate_api(api), Ok(format(code)))
    }

    #[test]
    fn test_should_generate_callback_with_no_return() {
        let mut api = Api::default();
        api.callbacks.push(Callback {
            return_type: FundamentalType("void".into()),
            pointer: None,
            name: "FMOD_FILE_ASYNCDONE_FUNC".into(),
            arguments: vec![Argument {
                as_const: None,
                argument_type: UserType("FMOD_ASYNCREADINFO".into()),
                pointer: normal(),
                name: "info".to_string(),
            }],
            varargs: None,
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

            pub type FMOD_FILE_ASYNCDONE_FUNC =
                Option<unsafe extern "C" fn(info: *mut FMOD_ASYNCREADINFO)>;
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_callback_with_varargs() {
        let mut api = Api::default();
        api.callbacks.push(Callback {
            return_type: FundamentalType("void".into()),
            pointer: None,
            name: "FMOD_DSP_LOG_FUNC".into(),
            arguments: vec![Argument {
                as_const: None,
                argument_type: UserType("FMOD_DEBUG_FLAGS".into()),
                pointer: None,
                name: "level".to_string(),
            }],
            varargs: Some("...".into()),
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

            pub type FMOD_DSP_LOG_FUNC =
                Option<unsafe extern "C" fn(level: FMOD_DEBUG_FLAGS, ...)>;
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_callback_with_keyword_argument() {
        let mut api = Api::default();
        api.callbacks.push(Callback {
            return_type: FundamentalType("void".into()),
            pointer: normal(),
            name: "FMOD_MEMORY_ALLOC_CALLBACK".into(),
            arguments: vec![
                Argument {
                    as_const: None,
                    argument_type: FundamentalType("unsigned int".into()),
                    pointer: None,
                    name: "size".to_string(),
                },
                Argument {
                    as_const: None,
                    argument_type: UserType("FMOD_MEMORY_TYPE".into()),
                    pointer: None,
                    name: "type".into(),
                },
            ],
            varargs: None,
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_uint, c_ulonglong, c_void};

            pub type FMOD_MEMORY_ALLOC_CALLBACK =
                Option<unsafe extern "C" fn(size: c_uint, type_: FMOD_MEMORY_TYPE) -> *mut c_void>;
        };

        assert_eq!(generate_api(api), Ok(format(code)));
    }
}
