use crate::models::{
    Argument, Callback, Constant, Enumeration, Error, Field, Flags, Function, OpaqueType, Pointer,
    Structure, Type, TypeAlias, Union,
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

pub fn format_rust_type(
    c_type: &Type,
    as_const: &Option<String>,
    pointer: &Option<Pointer>,
    as_array: &Option<TokenStream>,
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
            _ => name,
        },
        Type::UserType(name) => name,
    };
    let name = format_ident!("{}", name);
    let tokens = match (as_const, pointer) {
        (None, None) => quote! { #name },
        (None, Some(Pointer::NormalPointer(_))) => quote! { *mut #name },
        (None, Some(Pointer::DoublePointer(_))) => quote! { *mut *mut #name },
        (Some(_), Some(Pointer::NormalPointer(_))) => quote! { *const #name },
        (Some(_), Some(Pointer::DoublePointer(_))) => quote! { *const *const #name },
        (Some(_), None) => quote! { #name },
    };
    match as_array {
        None => tokens,
        Some(dimension) => {
            quote! {
                [#tokens; #dimension as usize]
            }
        }
    }
}

pub fn generate_type_alias_code(type_alias: &TypeAlias) -> TokenStream {
    let name = format_ident!("{}", type_alias.name);
    let base = format_rust_type(&type_alias.base_type, &None, &None, &None);

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

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "try", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield",
];

pub fn format_rust_ident(name: &String) -> Ident {
    if KEYWORDS.contains(&&*name.to_lowercase()) {
        format_ident!("{}_", name)
    } else {
        format_ident!("{}", name)
    }
}

pub fn generate_argument_code(argument: &Argument) -> TokenStream {
    let name = format_rust_ident(&argument.name);
    let argument_type = format_rust_type(
        &argument.argument_type,
        &argument.as_const,
        &argument.pointer,
        &None,
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
        let return_type = format_rust_type(&callback.return_type, &None, &callback.pointer, &None);
        quote! {
            pub type #name = Option<unsafe extern "C" fn(#(#arguments),* #varargs) -> #return_type>;
        }
    }
}

pub fn generate_flags_code(flags: &Flags) -> Result<TokenStream, Error> {
    let name = format_ident!("{}", flags.name);
    let base_type = format_rust_type(&flags.flags_type, &None, &None, &None);
    let mut values = vec![];
    for flag in &flags.flags {
        let value = TokenStream::from_str(&flag.value)?;
        let flag = format_ident!("{}", flag.name);
        values.push(quote! {
            pub const #flag: #name = #value;
        })
    }
    Ok(quote! {
        pub type #name = #base_type;
        #(#values)*
    })
}

pub fn generate_field_code(field: &Field) -> Result<TokenStream, Error> {
    let name = format_rust_ident(&field.name);
    let as_array = match &field.as_array {
        None => None,
        Some(dimension) => {
            let dimension = TokenStream::from_str(&dimension[1..dimension.len() - 1])?;
            Some(dimension)
        }
    };
    let field_type = format_rust_type(
        &field.field_type,
        &field.as_const,
        &field.pointer,
        &as_array,
    );
    Ok(quote! {
        pub #name: #field_type
    })
}

pub fn generate_structure_code(structure: &Structure) -> Result<TokenStream, Error> {
    let name = format_ident!("{}", structure.name);

    let mut fields = vec![];
    for field in &structure.fields {
        fields.push(generate_field_code(field)?);
    }

    let union = match &structure.union {
        None => None,
        Some(union) => {
            let name = format_ident!("{}__union", structure.name);
            fields.push(quote! {
                pub __union: #name
            });
            let mut fields = vec![];
            for field in &union.fields {
                fields.push(generate_field_code(field)?);
            }
            Some(quote! {
                #[repr(C)]
                #[derive(Copy, Clone)]
                pub union #name {
                    #(#fields),*
                }
            })
        }
    };

    let debug = if structure.union.is_some() {
        None
    } else {
        Some(quote! {Debug,})
    };

    Ok(quote! {
        #[repr(C)]
        #[derive(#debug Copy, Clone)]
        pub struct #name {
            #(#fields),*
        }
        #union
    })
}

pub fn generate_library_code(link: &String, api: &Vec<Function>) -> TokenStream {
    let mut functions = vec![];
    for function in api {
        let name = format_ident!("{}", function.name);
        let arguments: Vec<TokenStream> = function
            .arguments
            .iter()
            .map(generate_argument_code)
            .collect();

        let tokens = if &function.return_type == &FundamentalType("void".into()) {
            quote! {
               pub fn #name(#(#arguments),*);
            }
        } else {
            let return_type = format_rust_type(&function.return_type, &None, &None, &None);
            quote! {
                pub fn #name(#(#arguments),*) -> #return_type;
            }
        };
        functions.push(tokens);
    }
    quote! {
        #[link(name = #link)]
        extern "C" {
            #(#functions)*
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

    let mut flags = vec![];
    for flag in &api.flags {
        flags.push(generate_flags_code(flag)?);
    }

    let mut structures = vec![];
    for structure in &api.structures {
        structures.push(generate_structure_code(structure)?);
    }

    let mut libraries = vec![];
    for (link, functions) in &api.functions {
        libraries.push(generate_library_code(link, functions));
    }

    Ok(quote! {
        #![allow(non_camel_case_types)]
        #![allow(non_snake_case)]
        use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

        #(#opaque_types)*
        #(#type_aliases)*
        #(#constants)*
        #(#enumerations)*
        #(#flags)*
        #(#structures)*
        #(#callbacks)*
        #(#libraries)*
    })
}

pub fn generate_api(api: Api) -> Result<String, Error> {
    let code = generate_api_code(api)?;
    rustfmt_wrapper::rustfmt(code).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use crate::ffi::{generate_api, Api};
    use crate::models::Pointer::DoublePointer;
    use crate::models::Type::{FundamentalType, UserType};
    use crate::models::{
        Argument, Callback, Constant, Enumeration, Enumerator, Field, Flag, Flags, Function,
        OpaqueType, Pointer, Structure, TypeAlias, Union,
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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

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
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_MEMORY_ALLOC_CALLBACK =
                Option<unsafe extern "C" fn(size: c_uint, type_: FMOD_MEMORY_TYPE) -> *mut c_void>;
        };

        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_flags() {
        let mut api = Api::default();
        api.flags.push(Flags {
            flags_type: FundamentalType("unsigned int".into()),
            name: "FMOD_DEBUG_FLAGS".into(),
            flags: vec![
                Flag {
                    name: "FMOD_DEBUG_LEVEL_NONE".into(),
                    value: "0x00000000".into(),
                },
                Flag {
                    name: "FMOD_DEBUG_LEVEL_ERROR".into(),
                    value: "0x00000001".into(),
                },
            ],
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_DEBUG_FLAGS = c_uint;
            pub const FMOD_DEBUG_LEVEL_NONE: FMOD_DEBUG_FLAGS = 0x00000000;
            pub const FMOD_DEBUG_LEVEL_ERROR: FMOD_DEBUG_FLAGS = 0x00000001;
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_flags_with_bitwise_calculation() {
        let mut api = Api::default();
        api.flags.push(Flags {
            flags_type: FundamentalType("unsigned int".into()),
            name: "FMOD_CHANNELMASK".into(),
            flags: vec![
                Flag {
                    name: "FMOD_CHANNELMASK_FRONT_LEFT".into(),
                    value: "0x00000001".into(),
                },
                Flag {
                    name: "FMOD_CHANNELMASK_FRONT_RIGHT".into(),
                    value: "0x00000002".into(),
                },
                Flag {
                    name: "FMOD_CHANNELMASK_MONO".into(),
                    value: "(FMOD_CHANNELMASK_FRONT_LEFT)".into(),
                },
                Flag {
                    name: "FMOD_CHANNELMASK_STEREO".into(),
                    value: "(FMOD_CHANNELMASK_FRONT_LEFT | FMOD_CHANNELMASK_FRONT_RIGHT)".into(),
                },
            ],
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_CHANNELMASK = c_uint;
            pub const FMOD_CHANNELMASK_FRONT_LEFT: FMOD_CHANNELMASK = 0x00000001;
            pub const FMOD_CHANNELMASK_FRONT_RIGHT: FMOD_CHANNELMASK = 0x00000002;
            pub const FMOD_CHANNELMASK_MONO: FMOD_CHANNELMASK = (FMOD_CHANNELMASK_FRONT_LEFT);
            pub const FMOD_CHANNELMASK_STEREO: FMOD_CHANNELMASK = (FMOD_CHANNELMASK_FRONT_LEFT | FMOD_CHANNELMASK_FRONT_RIGHT);
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_flags_with_arithmetic_calculation() {
        let mut api = Api::default();
        api.flags.push(Flags {
            flags_type: FundamentalType("int".into()),
            name: "FMOD_THREAD_PRIORITY".into(),
            flags: vec![
                Flag {
                    name: "FMOD_THREAD_PRIORITY_PLATFORM_MIN".into(),
                    value: "(-32 * 1024)".into(),
                },
                Flag {
                    name: "FMOD_THREAD_PRIORITY_PLATFORM_MAX".into(),
                    value: "( 32 * 1024)".into(),
                },
            ],
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_THREAD_PRIORITY = c_int;
            pub const FMOD_THREAD_PRIORITY_PLATFORM_MIN: FMOD_THREAD_PRIORITY = (-32 * 1024);
            pub const FMOD_THREAD_PRIORITY_PLATFORM_MAX: FMOD_THREAD_PRIORITY = ( 32 * 1024);
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_structure() {
        let mut api = Api::default();
        api.structures.push(Structure {
            name: "FMOD_VECTOR".into(),
            fields: vec![
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("float".into()),
                    pointer: None,
                    name: "x".into(),
                },
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("float".into()),
                    pointer: None,
                    name: "y".into(),
                },
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("float".into()),
                    pointer: None,
                    name: "z".into(),
                },
            ],
            union: None,
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_VECTOR {
                pub x: c_float,
                pub y: c_float,
                pub z: c_float
            }
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_structure_with_array_field() {
        let mut api = Api::default();
        api.structures.push(Structure {
            name: "FMOD_GUID".into(),
            fields: vec![
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("unsigned int".into()),
                    pointer: None,
                    name: "Data1".into(),
                },
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("unsigned short".into()),
                    pointer: None,
                    name: "Data2".into(),
                },
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("unsigned short".into()),
                    pointer: None,
                    name: "Data3".into(),
                },
                Field {
                    as_const: None,
                    as_array: Some("[8]".into()),
                    field_type: FundamentalType("unsigned char".into()),
                    pointer: None,
                    name: "Data4".into(),
                },
            ],
            union: None,
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_GUID {
                pub Data1: c_uint,
                pub Data2: c_ushort,
                pub Data3: c_ushort,
                pub Data4: [c_uchar; 8 as usize],
            }
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_structure_with_array_const_dimension_field() {
        let mut api = Api::default();
        api.structures.push(Structure {
            name: "FMOD_DSP_LOUDNESS_METER_INFO_TYPE".into(),
            fields: vec![Field {
                as_const: None,
                as_array: Some("[FMOD_DSP_LOUDNESS_METER_HISTOGRAM_SAMPLES]".into()),
                field_type: FundamentalType("float".into()),
                pointer: None,
                name: "loudnesshistogram".into(),
            }],
            union: None,
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_DSP_LOUDNESS_METER_INFO_TYPE {
                pub loudnesshistogram: [c_float; FMOD_DSP_LOUDNESS_METER_HISTOGRAM_SAMPLES as usize],
            }
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_structure_with_const_char_const_field() {
        let mut api = Api::default();
        api.structures.push(Structure {
            name: "FMOD_DSP_PARAMETER_DESC_INT".into(),
            fields: vec![Field {
                as_const: Some("const".into()),
                as_array: None,
                field_type: FundamentalType("char".into()),
                pointer: Some(DoublePointer("* const*".into())),
                name: "valuenames".into(),
            }],
            union: None,
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_DSP_PARAMETER_DESC_INT {
                pub valuenames: *const *const c_char,
            }
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_structure_with_union() {
        let mut api = Api::default();
        api.structures.push(Structure {
            name: "FMOD_DSP_PARAMETER_DESC".into(),
            fields: vec![Field {
                as_const: None,
                as_array: None,
                field_type: UserType("FMOD_DSP_PARAMETER_TYPE".into()),
                pointer: None,
                name: "type".into(),
            }],
            union: Some(Union {
                fields: vec![
                    Field {
                        as_const: None,
                        as_array: None,
                        field_type: UserType("FMOD_DSP_PARAMETER_DESC_FLOAT".into()),
                        pointer: None,
                        name: "floatdesc".into(),
                    },
                    Field {
                        as_const: None,
                        as_array: None,
                        field_type: UserType("FMOD_DSP_PARAMETER_DESC_INT".into()),
                        pointer: None,
                        name: "intdesc".into(),
                    },
                ],
            }),
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Copy, Clone)]
            pub struct FMOD_DSP_PARAMETER_DESC {
                pub type_: FMOD_DSP_PARAMETER_TYPE,
                pub __union: FMOD_DSP_PARAMETER_DESC__union,
            }
            #[repr(C)]
            #[derive(Copy, Clone)]
            pub union FMOD_DSP_PARAMETER_DESC__union {
                pub floatdesc: FMOD_DSP_PARAMETER_DESC_FLOAT,
                pub intdesc: FMOD_DSP_PARAMETER_DESC_INT,
            }
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_function() {
        let mut api = Api::default();
        api.functions.insert(
            "fmod".into(),
            vec![Function {
                return_type: UserType("FMOD_RESULT".into()),
                name: "FMOD_System_Create".into(),
                arguments: vec![
                    Argument {
                        as_const: None,
                        argument_type: UserType("FMOD_SYSTEM".into()),
                        pointer: Some(DoublePointer("**".into())),
                        name: "system".into(),
                    },
                    Argument {
                        as_const: None,
                        argument_type: FundamentalType("unsigned int".into()),
                        pointer: None,
                        name: "headerversion".into(),
                    },
                ],
            }],
        );
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[link(name = "fmod")]
            extern "C" {
                pub fn FMOD_System_Create(
                    system: *mut *mut FMOD_SYSTEM,
                    headerversion: c_uint,
                ) -> FMOD_RESULT;
            }
        };
        assert_eq!(generate_api(api), Ok(format(code)));
    }
}
