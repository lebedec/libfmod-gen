use crate::models::Pointer::DoublePointer;
use crate::models::Type::{FundamentalType, UserType};
use crate::models::{Api, Argument, Enumeration, Error, Function, Pointer, Structure, Type};
use convert_case::{Case, Casing};
use quote::__private::{Ident, TokenStream};
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::generators::dictionary::RENAMES;

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub structure: Structure,
    pub constructor: Function,
    pub methods: Vec<Function>,
}

#[derive(Debug, Default)]
pub struct Lib {
    pub structs: Vec<Struct>,
}

fn extract_struct_key(name: &str) -> String {
    match name.rfind('_') {
        Some(index) => name[..index].to_uppercase(),
        None => name.to_string(),
    }
}

const ENUMERATOR_RENAMES: &[(&str, &str)] = &[
    ("FMOD_STUDIO_LOAD_MEMORY", "FMOD_STUDIO_LOAD_MEMORY_MMEMORY"),
    (
        "FMOD_STUDIO_LOAD_MEMORY_POINT",
        "FMOD_STUDIO_LOAD_MEMORY_MMEMORY_POINT",
    ),
];

fn format_enumerator_ident(enumeration: &str, name: &str) -> Ident {
    let name = match ENUMERATOR_RENAMES.iter().find(|pair| pair.0 == name) {
        None => name,
        Some(pair) => pair.1,
    };
    let mut p = 0;
    while p < name.len() && p < enumeration.len() {
        if enumeration.chars().nth(p) != name.chars().nth(p) {
            break;
        }
        p += 1;
    }
    let key = (&name[p..]).to_case(Case::UpperCamel);

    let key = if key.chars().nth(0).unwrap_or('a').is_ascii_digit() {
        format!("_{}", key)
    } else {
        key
    };
    let name = key;
    let name = match RENAMES.get(&name[..]) {
        None => name,
        Some(rename) => rename.to_string(),
    };
    format_ident!("{}", name)
}

fn extract_method_name(name: &str) -> String {
    match name.rfind('_') {
        Some(index) => name[index..].to_string().to_case(Case::Snake),
        None => name.to_string(),
    }
}

fn format_struct_ident(key: &str) -> Ident {
    let key = key.replace("FMOD_", "");
    let key = key.replace("STUDIO_SYSTEM", "STUDIOSYSTEM");
    let key = key.replace("STUDIO_", "");
    let name = key.to_case(Case::Pascal);
    let name = match RENAMES.get(&name[..]) {
        None => name,
        Some(rename) => rename.to_string(),
    };
    format_ident!("{}", name)
}

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "try", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield",
];

pub fn format_argument_ident(name: &str) -> Ident {
    let name = name.to_case(Case::Snake);
    if KEYWORDS.contains(&&*name) {
        format_ident!("{}_", name)
    } else {
        format_ident!("{}", name)
    }
}

pub fn generate_struct_method_code(method: &Function) -> TokenStream {
    quote! {
        pub fn load_bank_file(
            &self,
            filename: &str,
            flags: FMOD_STUDIO_LOAD_BANK_FLAGS,
        ) -> Result<Bank, MyError> {
            let mut pointer = null_mut();
            let filename = CString::new(filename).unwrap();
            let result = unsafe {
                FMOD_Studio_System_LoadBankFile(
                    self.pointer,
                    filename.as_ptr(),
                    flags,
                    &mut pointer,
                )
            };
            if result == FMOD_OK {
                Ok(pointer.into())
            } else {
                Err(MyError("FMOD_Studio_System_LoadBankFile".into(), decode_error(result).to_string()))
            }
        }
    }
}

pub fn is_normal(pointer: &Option<Pointer>) -> bool {
    if let Some(Pointer::NormalPointer(_)) = pointer {
        true
    } else {
        false
    }
}

pub fn is_double(pointer: &Option<Pointer>) -> bool {
    if let Some(Pointer::DoublePointer(_)) = pointer {
        true
    } else {
        false
    }
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
            "int" => "i32",
            "unsigned int" => "u32",
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

pub fn generate_argument_code(argument: &Argument) -> TokenStream {
    let name = format_argument_ident(&argument.name);
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

pub fn generate_argument_map_code(argument: &Argument) -> TokenStream {
    let name = format_argument_ident(&argument.name);
    quote! { #name }
}

pub fn generate_enumeration_code(enumeration: &Enumeration) -> TokenStream {
    let name = format_struct_ident(&enumeration.name);

    let mut keys = vec![];
    let mut input_map = vec![];
    let mut output_map = vec![];

    for enumerator in &enumeration.enumerators {
        if enumerator.name.ends_with("FORCEINT") {
            // skip unused workaround
            continue;
        }
        let key = format_enumerator_ident(&enumeration.name, &enumerator.name);
        let enumerator = format_ident!("{}", enumerator.name);
        input_map.push(quote! {#name::#key => ffi::#enumerator});
        output_map.push(quote! {ffi::#enumerator => Ok(#name::#key)});
        keys.push(key);
    }

    let enumeration_name = &enumeration.name;
    let enumeration = format_ident!("{}", enumeration_name);

    quote! {
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum #name {
            #(#keys),*
        }

        impl From<#name> for ffi::#enumeration {
            fn from(value: #name) -> ffi::#enumeration {
                match value {
                    #(#input_map),*
                }
            }
        }

        impl #name {
            pub fn from(value: ffi::#enumeration) -> Result<#name, Error> {
                match value {
                    #(#output_map),*,
                    _ => Err(err_enum!(#enumeration_name, value)),
                }
            }
        }
    }
}

pub fn generate_method_code(owner: &str, function: &Function) -> TokenStream {
    let mut arguments = vec![];
    let mut output = None;

    for argument in function.arguments.clone() {
        if argument.argument_type == UserType(owner.into()) && is_normal(&argument.pointer) {
            continue;
        } else if is_double(&argument.pointer) && output.is_none() {
            output = Some(argument)
        } else {
            arguments.push(argument);
        }
    }

    let argument_maps: Vec<TokenStream> =
        arguments.iter().map(generate_argument_map_code).collect();
    let arguments: Vec<TokenStream> = arguments.iter().map(generate_argument_code).collect();

    let method = format_ident!(
        "{}",
        extract_method_name(&function.name).to_case(Case::Snake)
    );
    let function_name = &function.name;
    let function = format_ident!("{}", function_name);

    match output {
        Some(_) => quote! {
            pub fn #method(&self, #(#arguments),*) -> Result<Bank, Error> {
                let mut output = null_mut();
                let filename = CString::new(filename).unwrap();
                let result = unsafe {
                    ffi::#function(self.pointer, &mut output)
                };
                if result == FMOD_OK {
                    Ok(output.into())
                } else {
                    Err(err_fmod!(#function_name, result))
                }
            }
        },
        None => quote! {
            pub fn #method(&self, #(#arguments),*) -> Result<(), Error> {
                let result = unsafe {
                    ffi::#function(self.pointer, #(#argument_maps),*)
                };
                if result == FMOD_OK {
                    Ok(())
                } else {
                    Err(err_fmod!(#function_name, result))
                }
            }
        },
    }
}

pub fn generate_struct_code(key: &String, methods: &Vec<&Function>) -> TokenStream {
    let name = format_struct_ident(key);
    let opaque_type = format_ident!("{}", key);

    let constructor = match methods.iter().find(|method| {
        method.name.ends_with("Create")
            && method.arguments.iter().any(|argument| {
                argument.argument_type == UserType(key.clone())
                    && argument.pointer == Some(DoublePointer("**".into()))
            })
    }) {
        None => None,
        Some(function) => {
            let name = format_ident!("{}", extract_method_name(&function.name));
            let function_name = &function.name;
            let function = format_ident!("{}", function_name);
            Some(quote! {
                pub fn #name() -> Result<Self, Error> {
                    let mut pointer = null_mut();
                    let result = unsafe {
                        ffi::#function(&mut pointer, ffi::FMOD_VERSION)
                    };
                    if result == ffi::FMOD_OK {
                        Ok(Self { pointer })
                    } else {
                        Err(err_fmod!(#function_name, result))
                    }
                }
            })
        }
    };

    quote! {
        pub struct #name {
            pointer: *mut ffi::#opaque_type,
        }

        impl #name {
            #constructor
        }
    }
}

pub fn generate_lib_code(api: &Api) -> Result<TokenStream, Error> {
    let functions: Vec<&Function> = api
        .functions
        .iter()
        .flat_map(|(_, functions)| functions)
        .collect();

    let opaque_types = api
        .opaque_types
        .iter()
        .map(|opaque_type| opaque_type.name.clone());
    let opaque_types: HashSet<String> = HashSet::from_iter(opaque_types);

    let mut structs: BTreeMap<String, Vec<&Function>> = BTreeMap::new();
    for function in &functions {
        let key = extract_struct_key(&function.name);
        if opaque_types.contains(&key) {
            match structs.get_mut(&key) {
                Some(methods) => methods.push(function),
                None => {
                    structs.insert(key, vec![function]);
                }
            }
        } else {
            println!("Global function: {}", function.name);
        }
    }

    let structs: Vec<TokenStream> = structs
        .iter()
        .map(|(key, methods)| generate_struct_code(key, methods))
        .collect();

    let enumerations: Vec<TokenStream> = api
        .enumerations
        .iter()
        .filter(|enumeration| enumeration.name != "FMOD_RESULT")
        .map(generate_enumeration_code)
        .collect();

    Ok(quote! {
        use std::ptr::null_mut;
        pub mod ffi;

        #[derive(Debug)]
        pub enum Error {
            Fmod {
                function: String,
                code: i32,
                message: String,
            },
            EnumBindgen {
                enumeration: String,
                value: String
            }
        }

        macro_rules! err_fmod {
            ($ function : expr , $ code : expr) => {
                Error::Fmod {
                    function: $function.to_string(),
                    code: $code,
                    message: ffi::map_fmod_error($code).to_string(),
                }
            };
        }

        macro_rules! err_enum {
            ($ enumeration : expr , $ value : expr) => {
                Error::EnumBindgen {
                    enumeration: $enumeration.to_string(),
                    value: $value.to_string(),
                }
            };
        }

        #(#enumerations)*

        #(#structs)*
    })
}

pub fn generate(api: &Api) -> Result<String, Error> {
    let code = generate_lib_code(api)?;
    rustfmt_wrapper::rustfmt(code).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use crate::lib::{generate_enumeration_code, generate_method_code};
    use crate::models::Type::{FundamentalType, UserType};
    use crate::models::{Argument, Enumeration, Enumerator, Function, Pointer};

    fn normal() -> Option<Pointer> {
        Some(Pointer::NormalPointer("*".into()))
    }

    #[test]
    fn test_should_generate_simple_arguments_method() {
        let function = Function {
            return_type: UserType("FMOD_RESULT".into()),
            name: "FMOD_System_SetDSPBufferSize".to_string(),
            arguments: vec![
                Argument {
                    as_const: None,
                    argument_type: UserType("FMOD_SYSTEM".into()),
                    pointer: normal(),
                    name: "system".to_string(),
                },
                Argument {
                    as_const: None,
                    argument_type: FundamentalType("unsigned int".into()),
                    pointer: None,
                    name: "bufferlength".to_string(),
                },
                Argument {
                    as_const: None,
                    argument_type: FundamentalType("int".into()),
                    pointer: None,
                    name: "numbuffers".to_string(),
                },
            ],
        };
        let actual = generate_method_code("FMOD_SYSTEM", &function).to_string();
        let expected = quote! {
            pub fn set_dsp_buffer_size(&self, bufferlength: u32, numbuffers: i32) -> Result<(), Error> {
                let result = unsafe {
                    ffi::FMOD_System_SetDSPBufferSize(self.pointer, bufferlength, numbuffers)
                };
                if result == FMOD_OK {
                    Ok(())
                } else {
                    Err(err_fmod!("FMOD_System_SetDSPBufferSize", result))
                }
            }
        }.to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_should_generate_enumeration() {
        let enumeration = Enumeration {
            name: "FMOD_OUTPUTTYPE".to_string(),
            enumerators: vec![
                Enumerator {
                    name: "FMOD_OUTPUTTYPE_AUTODETECT".to_string(),
                    value: None,
                },
                Enumerator {
                    name: "FMOD_OUTPUTTYPE_UNKNOWN".to_string(),
                    value: None,
                },
                Enumerator {
                    name: "FMOD_OUTPUTTYPE_FORCEINT".to_string(),
                    value: Some("65536".into()),
                },
            ],
        };
        let actual = generate_enumeration_code(&enumeration).to_string();
        let expected = quote! {
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub enum OutputType {
                Autodetect,
                Unknown
            }

            impl From<OutputType> for ffi::FMOD_OUTPUTTYPE {
                fn from(value: OutputType) -> ffi::FMOD_OUTPUTTYPE {
                    match value {
                        OutputType::Autodetect => ffi::FMOD_OUTPUTTYPE_AUTODETECT,
                        OutputType::Unknown => ffi::FMOD_OUTPUTTYPE_UNKNOWN
                    }
                }
            }

            impl OutputType {
                pub fn from(value: ffi::FMOD_OUTPUTTYPE) -> Result<OutputType, Error> {
                    match value {
                        ffi::FMOD_OUTPUTTYPE_AUTODETECT => Ok(OutputType::Autodetect),
                        ffi::FMOD_OUTPUTTYPE_UNKNOWN => Ok(OutputType::Unknown),
                        _ => Err(err_enum!("FMOD_OUTPUTTYPE" , value)),
                    }
                }
            }
        }
        .to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_should_generate_enumeration_with_digits() {
        let enumeration = Enumeration {
            name: "FMOD_SPEAKERMODE".to_string(),
            enumerators: vec![
                Enumerator {
                    name: "FMOD_SPEAKERMODE_DEFAULT".to_string(),
                    value: None,
                },
                Enumerator {
                    name: "FMOD_SPEAKERMODE_5POINT1".to_string(),
                    value: None,
                },
            ],
        };
        let actual = generate_enumeration_code(&enumeration).to_string();
        let expected = quote! {
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub enum SpeakerMode {
                Default,
                _5Point1
            }

            impl From<SpeakerMode> for ffi::FMOD_SPEAKERMODE {
                fn from(value: SpeakerMode) -> ffi::FMOD_SPEAKERMODE {
                    match value {
                        SpeakerMode::Default => ffi::FMOD_SPEAKERMODE_DEFAULT,
                        SpeakerMode::_5Point1 => ffi::FMOD_SPEAKERMODE_5POINT1
                    }
                }
            }

            impl SpeakerMode {
                pub fn from(value: ffi::FMOD_SPEAKERMODE) -> Result<SpeakerMode, Error> {
                    match value {
                        ffi::FMOD_SPEAKERMODE_DEFAULT => Ok(SpeakerMode::Default),
                        ffi::FMOD_SPEAKERMODE_5POINT1 => Ok(SpeakerMode::_5Point1),
                        _ => Err(err_enum!("FMOD_SPEAKERMODE" , value)),
                    }
                }
            }
        }
        .to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_should_generate_enumeration_with_mismatch_names() {
        let enumeration = Enumeration {
            name: "FMOD_STUDIO_PARAMETER_TYPE".to_string(),
            enumerators: vec![
                Enumerator {
                    name: "FMOD_STUDIO_PARAMETER_GAME_CONTROLLED".to_string(),
                    value: None,
                },
                Enumerator {
                    name: "FMOD_STUDIO_PARAMETER_AUTOMATIC_DISTANCE".to_string(),
                    value: None,
                },
            ],
        };
        let actual = generate_enumeration_code(&enumeration).to_string();
        let expected = quote! {
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub enum ParameterType {
                GameControlled,
                AutomaticDistance
            }

            impl From<ParameterType> for ffi::FMOD_STUDIO_PARAMETER_TYPE {
                fn from(value: ParameterType) -> ffi::FMOD_STUDIO_PARAMETER_TYPE {
                    match value {
                        ParameterType::GameControlled => ffi::FMOD_STUDIO_PARAMETER_GAME_CONTROLLED,
                        ParameterType::AutomaticDistance => ffi::FMOD_STUDIO_PARAMETER_AUTOMATIC_DISTANCE
                    }
                }
            }

            impl ParameterType {
                pub fn from(value: ffi::FMOD_STUDIO_PARAMETER_TYPE) -> Result<ParameterType, Error> {
                    match value {
                        ffi::FMOD_STUDIO_PARAMETER_GAME_CONTROLLED => Ok(ParameterType::GameControlled),
                        ffi::FMOD_STUDIO_PARAMETER_AUTOMATIC_DISTANCE => Ok(ParameterType::AutomaticDistance),
                        _ => Err(err_enum!("FMOD_STUDIO_PARAMETER_TYPE" , value)),
                    }
                }
            }
        }
        .to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_should_generate_enumeration_with_short_enumerator() {
        let enumeration = Enumeration {
            name: "FMOD_STUDIO_LOAD_MEMORY_MODE".to_string(),
            enumerators: vec![
                Enumerator {
                    name: "FMOD_STUDIO_LOAD_MEMORY".to_string(),
                    value: None,
                },
                Enumerator {
                    name: "FMOD_STUDIO_LOAD_MEMORY_POINT".to_string(),
                    value: None,
                },
            ],
        };
        let actual = generate_enumeration_code(&enumeration).to_string();
        let expected = quote! {
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub enum LoadMemoryMode {
                Memory,
                MemoryPoint
            }

            impl From<LoadMemoryMode> for ffi::FMOD_STUDIO_LOAD_MEMORY_MODE {
                fn from(value: LoadMemoryMode) -> ffi::FMOD_STUDIO_LOAD_MEMORY_MODE {
                    match value {
                        LoadMemoryMode::Memory => ffi::FMOD_STUDIO_LOAD_MEMORY,
                        LoadMemoryMode::MemoryPoint => ffi::FMOD_STUDIO_LOAD_MEMORY_POINT
                    }
                }
            }

            impl LoadMemoryMode {
                pub fn from(value: ffi::FMOD_STUDIO_LOAD_MEMORY_MODE) -> Result<LoadMemoryMode, Error> {
                    match value {
                        ffi::FMOD_STUDIO_LOAD_MEMORY => Ok(LoadMemoryMode::Memory),
                        ffi::FMOD_STUDIO_LOAD_MEMORY_POINT => Ok(LoadMemoryMode::MemoryPoint),
                        _ => Err(err_enum!("FMOD_STUDIO_LOAD_MEMORY_MODE" , value)),
                    }
                }
            }
        }
        .to_string();
        assert_eq!(actual, expected);
    }
}
