use std::num::{ParseFloatError, ParseIntError};
use std::str::FromStr;

use quote::__private::{Ident, LexError, Literal, TokenStream};
use quote::quote;

use crate::models::Type::{FundamentalType, UserType};
use crate::models::{
    Api, Argument, Callback, Constant, Enumeration, Error, ErrorStringMapping, Field, Flags,
    Function, OpaqueType, Pointer, Preset, Structure, Type, TypeAlias,
};

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

impl From<ParseFloatError> for Error {
    fn from(error: ParseFloatError) -> Self {
        Error::ParseFloat(error.to_string())
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

pub fn describe_ffi_pointer<'a>(
    as_const: &'a Option<String>,
    pointer: &'a Option<Pointer>,
) -> &'a str {
    let description = match (as_const, pointer) {
        (None, None) => "",
        (None, Some(Pointer::NormalPointer(_))) => "*mut",
        (None, Some(Pointer::DoublePointer(_))) => "*mut *mut",
        (Some(_), Some(Pointer::NormalPointer(_))) => "*const",
        (Some(_), Some(Pointer::DoublePointer(_))) => "*const *const",
        (Some(_), None) => "",
    };
    description
}

pub fn generate_field_default(owner: &str, field: &Field) -> Result<TokenStream, Error> {
    let name = format_rust_ident(&field.name);
    let ptr = describe_ffi_pointer(&field.as_const, &field.pointer);

    let tokens = match (owner, &field.name[..]) {
        ("FMOD_STUDIO_ADVANCEDSETTINGS", "cbsize") => {
            quote! { size_of::<FMOD_STUDIO_ADVANCEDSETTINGS>() as i32 }
        }
        ("FMOD_ADVANCEDSETTINGS", "cbSize") => {
            quote! { size_of::<FMOD_ADVANCEDSETTINGS>() as i32 }
        }
        ("FMOD_CREATESOUNDEXINFO", "cbsize") => {
            quote! { size_of::<FMOD_CREATESOUNDEXINFO>() as i32 }
        }
        _ => match &field.field_type {
            FundamentalType(name) => match (ptr, &name[..]) {
                ("*mut", _) => quote! { null_mut() },
                ("*const", _) => quote! { null_mut() },
                ("*mut *mut", _) => quote! { null_mut() },
                ("*const *const", _) => quote! { null_mut() },
                _ => quote! { Default::default() },
            },
            UserType(_) => match ptr {
                "*mut" => quote! { null_mut() },
                "*mut *mut" => quote! { null_mut() },
                _ => quote! {  Default::default() },
            },
        },
    };

    Ok(quote! {
        #name: #tokens
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
    let mut defaults = vec![];
    for field in &structure.fields {
        fields.push(generate_field_code(field)?);
        defaults.push(generate_field_default(&structure.name, field)?);
    }

    let union = match &structure.union {
        None => None,
        Some(union) => {
            let name = format_ident!("{}_UNION", structure.name);
            fields.push(quote! {
                pub union: #name
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

    let unimplemented = vec![
        "FMOD_STUDIO_USER_PROPERTY",
        "FMOD_DSP_PARAMETER_DESC",
        "FMOD_DSP_LOUDNESS_METER_INFO_TYPE",
        "FMOD_DSP_PARAMETER_FFT",
    ];

    let default = if unimplemented.contains(&&*structure.name) {
        quote! {
            unimplemented!()
        }
    } else {
        quote! {
            Self {
                #(#defaults),*
            }
        }
    };

    Ok(quote! {
        #[repr(C)]
        #[derive(#debug Copy, Clone)]
        pub struct #name {
            #(#fields),*
        }
        #union
        impl Default for #name {
            fn default() -> Self {
                #default
            }
        }
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

pub fn generate_preset_code(structure: &Structure, preset: &Preset) -> Result<TokenStream, Error> {
    let name = format_ident!("{}", preset.name);
    let mut fields: Vec<TokenStream> = vec![];
    for (index, value) in preset.values.iter().enumerate() {
        let value = if value.ends_with("f") {
            &value[0..value.len() - 1]
        } else {
            &value[..]
        };
        let value: f32 = value.parse()?;
        let field = format_rust_ident(&structure.fields[index].name);
        let value = Literal::f32_unsuffixed(value);
        fields.push(quote! {
            #field: #value
        });
    }
    let structure = format_ident!("{}", structure.name);

    Ok(quote! {
        pub const #name: #structure = #structure {
            #(#fields),*
        };
    })
}

pub fn generate_errors_mapping_code(mapping: &ErrorStringMapping) -> TokenStream {
    let mut cases = vec![];
    for error in &mapping.errors {
        let result = format_ident!("{}", error.name);
        let error = &error.string;
        cases.push(quote! {
            #result => #error,
        });
    }
    quote! {
        pub fn map_fmod_error(result: FMOD_RESULT) -> &'static str {
            match result {
                #(#cases)*
                _ => "Unknown error code"
            }
        }
    }
}

pub fn generate_ffi_code(api: &Api) -> Result<TokenStream, Error> {
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

    let mut presets = vec![];
    if let Some(structure) = api
        .structures
        .iter()
        .find(|structure| structure.name == "FMOD_REVERB_PROPERTIES")
    {
        for preset in &api.presets {
            presets.push(generate_preset_code(structure, preset)?);
        }
    }

    let errors = if api.errors.errors.is_empty() {
        None
    } else {
        Some(generate_errors_mapping_code(&api.errors))
    };

    Ok(quote! {
        #![allow(non_camel_case_types)]
        #![allow(non_snake_case)]
        #![allow(unused_parens)]
        use std::mem::size_of;
        use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};
        use std::ptr::null_mut;

        #(#opaque_types)*
        #(#type_aliases)*
        #(#constants)*
        #(#enumerations)*
        #(#flags)*
        #(#structures)*
        #(#presets)*
        #(#callbacks)*
        #(#libraries)*
        #errors
    })
}

pub fn generate(api: &Api) -> Result<String, Error> {
    let code = generate_ffi_code(api)?;
    rustfmt_wrapper::rustfmt(code).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use quote::__private::TokenStream;

    use crate::ffi::{generate, Api};
    use crate::models::Pointer::DoublePointer;
    use crate::models::Type::{FundamentalType, UserType};
    use crate::models::{
        Argument, Callback, Constant, Enumeration, Enumerator, ErrorString, ErrorStringMapping,
        Field, Flag, Flags, Function, OpaqueType, Pointer, Preset, Structure, TypeAlias, Union,
    };

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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub const FMOD_MAX_CHANNEL_WIDTH: c_uint = 32;
        };
        assert_eq!(generate(&api), Ok(format(code)))
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub const FMOD_PORT_INDEX_NONE: c_ulonglong = 0xFFFFFFFFFFFFFFFF;
        };
        assert_eq!(generate(&api), Ok(format(code)))
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub const FMOD_VERSION: c_uint = 0x00020203;
        };
        assert_eq!(generate(&api), Ok(format(code)))
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_PORT_INDEX = c_ulonglong;
        };
        assert_eq!(generate(&api), Ok(format(code)))
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_SOUND {
                _unused: [u8; 0]
            }
        };
        assert_eq!(generate(&api), Ok(format(code)))
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
            #![allow(unused_parens)]
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
        assert_eq!(generate(&api), Ok(format(code)))
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_CHANNELCONTROL_DSP_INDEX = c_int;
            pub const FMOD_CHANNELCONTROL_DSP_HEAD: FMOD_CHANNELCONTROL_DSP_INDEX = -1;
            pub const FMOD_CHANNELCONTROL_DSP_FADER: FMOD_CHANNELCONTROL_DSP_INDEX = -2;
            pub const FMOD_CHANNELCONTROL_DSP_FORCEINT: FMOD_CHANNELCONTROL_DSP_INDEX = 65536;
        };
        assert_eq!(generate(&api), Ok(format(code)))
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_PLUGINTYPE = c_int;
            pub const FMOD_PLUGINTYPE_OUTPUT: FMOD_PLUGINTYPE = 0;
            pub const FMOD_PLUGINTYPE_CODEC: FMOD_PLUGINTYPE = 1;
        };
        assert_eq!(generate(&api), Ok(format(code)))
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_SPEAKER = c_int;
            pub const FMOD_SPEAKER_NONE: FMOD_SPEAKER = -1;
            pub const FMOD_SPEAKER_FRONT_LEFT: FMOD_SPEAKER = 0;
            pub const FMOD_SPEAKER_FRONT_RIGHT: FMOD_SPEAKER = 1;
            pub const FMOD_SPEAKER_FRONT_CENTER: FMOD_SPEAKER = 2;
            pub const FMOD_SPEAKER_FORCEINT: FMOD_SPEAKER = 65536;
        };
        assert_eq!(generate(&api), Ok(format(code)))
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_FILE_ASYNCDONE_FUNC =
                Option<unsafe extern "C" fn(info: *mut FMOD_ASYNCREADINFO)>;
        };
        assert_eq!(generate(&api), Ok(format(code)));
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_DSP_LOG_FUNC =
                Option<unsafe extern "C" fn(level: FMOD_DEBUG_FLAGS, ...)>;
        };
        assert_eq!(generate(&api), Ok(format(code)));
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_MEMORY_ALLOC_CALLBACK =
                Option<unsafe extern "C" fn(size: c_uint, type_: FMOD_MEMORY_TYPE) -> *mut c_void>;
        };

        assert_eq!(generate(&api), Ok(format(code)));
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_DEBUG_FLAGS = c_uint;
            pub const FMOD_DEBUG_LEVEL_NONE: FMOD_DEBUG_FLAGS = 0x00000000;
            pub const FMOD_DEBUG_LEVEL_ERROR: FMOD_DEBUG_FLAGS = 0x00000001;
        };
        assert_eq!(generate(&api), Ok(format(code)));
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_CHANNELMASK = c_uint;
            pub const FMOD_CHANNELMASK_FRONT_LEFT: FMOD_CHANNELMASK = 0x00000001;
            pub const FMOD_CHANNELMASK_FRONT_RIGHT: FMOD_CHANNELMASK = 0x00000002;
            pub const FMOD_CHANNELMASK_MONO: FMOD_CHANNELMASK = (FMOD_CHANNELMASK_FRONT_LEFT);
            pub const FMOD_CHANNELMASK_STEREO: FMOD_CHANNELMASK = (FMOD_CHANNELMASK_FRONT_LEFT | FMOD_CHANNELMASK_FRONT_RIGHT);
        };
        assert_eq!(generate(&api), Ok(format(code)));
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub type FMOD_THREAD_PRIORITY = c_int;
            pub const FMOD_THREAD_PRIORITY_PLATFORM_MIN: FMOD_THREAD_PRIORITY = (-32 * 1024);
            pub const FMOD_THREAD_PRIORITY_PLATFORM_MAX: FMOD_THREAD_PRIORITY = ( 32 * 1024);
        };
        assert_eq!(generate(&api), Ok(format(code)));
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_VECTOR {
                pub x: c_float,
                pub y: c_float,
                pub z: c_float
            }
        };
        assert_eq!(generate(&api), Ok(format(code)));
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
            #![allow(unused_parens)]
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
        assert_eq!(generate(&api), Ok(format(code)));
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_DSP_LOUDNESS_METER_INFO_TYPE {
                pub loudnesshistogram: [c_float; FMOD_DSP_LOUDNESS_METER_HISTOGRAM_SAMPLES as usize],
            }
        };
        assert_eq!(generate(&api), Ok(format(code)));
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_DSP_PARAMETER_DESC_INT {
                pub valuenames: *const *const c_char,
            }
        };
        assert_eq!(generate(&api), Ok(format(code)));
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
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Copy, Clone)]
            pub struct FMOD_DSP_PARAMETER_DESC {
                pub type_: FMOD_DSP_PARAMETER_TYPE,
                pub union: FMOD_DSP_PARAMETER_DESC_UNION,
            }
            #[repr(C)]
            #[derive(Copy, Clone)]
            pub union FMOD_DSP_PARAMETER_DESC_UNION {
                pub floatdesc: FMOD_DSP_PARAMETER_DESC_FLOAT,
                pub intdesc: FMOD_DSP_PARAMETER_DESC_INT,
            }
        };
        assert_eq!(generate(&api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_preset() {
        let mut api = Api::default();
        api.structures.push(Structure {
            name: "FMOD_REVERB_PROPERTIES".to_string(),
            fields: vec![
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("float".into()),
                    pointer: None,
                    name: "DecayTime".into(),
                },
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("float".into()),
                    pointer: None,
                    name: "EarlyDelay".into(),
                },
            ],
            union: None,
        });
        api.presets.push(Preset {
            name: "FMOD_PRESET_OFF".into(),
            values: vec!["96".into(), "-8.0f".into()],
        });
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            pub struct FMOD_REVERB_PROPERTIES {
                pub DecayTime: c_float,
                pub EarlyDelay: c_float,
            }

            pub const FMOD_PRESET_OFF: FMOD_REVERB_PROPERTIES = FMOD_REVERB_PROPERTIES {
                DecayTime: 96.0,
                EarlyDelay: -8.0,
            };
        };
        assert_eq!(generate(&api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_function() {
        let mut api = Api::default();
        api.functions.push((
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
        ));
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            #[link(name = "fmod")]
            extern "C" {
                pub fn FMOD_System_Create(
                    system: *mut *mut FMOD_SYSTEM,
                    headerversion: c_uint,
                ) -> FMOD_RESULT;
            }
        };
        assert_eq!(generate(&api), Ok(format(code)));
    }

    #[test]
    fn test_should_generate_error_mapping() {
        /*

        case FMOD_OK:                            return "No errors.";
        case FMOD_ERR_BADCOMMAND:                return "Tried to call a function on a data type that does not allow this type of functionality (ie calling Sound::lock on a streaming sound).";
        case FMOD_ERR_CHANNEL_ALLOC:             return "Error trying to allocate a channel.";
         */
        let mut api = Api::default();
        api.errors = ErrorStringMapping {
            errors: vec![
                ErrorString {
                    name: "FMOD_OK".into(),
                    string: "No errors.".into(),
                },
                ErrorString {
                    name: "FMOD_ERR_CHANNEL_ALLOC".into(),
                    string: "Error trying to allocate a channel.".into(),
                },
            ],
        };
        let code = quote! {
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            #![allow(unused_parens)]
            use std::os::raw::{c_char, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void};

            pub fn map_fmod_error(result: FMOD_RESULT) -> &'static str {
                match result {
                    FMOD_OK => "No errors.",
                    FMOD_ERR_CHANNEL_ALLOC => "Error trying to allocate a channel.",
                    _ => "Unknown error code"
                }
            }
        };
        assert_eq!(generate(&api), Ok(format(code)));
    }
}
