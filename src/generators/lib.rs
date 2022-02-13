use crate::models::Pointer::DoublePointer;
use crate::models::Type::UserType;
use crate::models::{Api, Error, Function, Structure};
use convert_case::{Case, Casing};
use quote::__private::{Ident, TokenStream};
use std::collections::{BTreeMap, HashMap, HashSet};

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

fn extract_method_name(name: &str) -> String {
    match name.rfind('_') {
        Some(index) => name[index..].to_string().to_case(Case::Snake),
        None => name.to_string(),
    }
}

fn format_struct_name(key: &str) -> Ident {
    let renames: HashMap<&str, &str> = HashMap::from([
        ("Channelgroup", "ChannelGroup"),
        ("Dspconnection", "DspConnection"),
        ("Reverb3D", "Reverb3d"),
        ("Soundgroup", "SoundGroup"),
        ("Commandreplay", "CommandReplay"),
        ("Eventdescription", "EventDescription"),
        ("Eventinstance", "EventInstance"),
        ("Studiosystem", "Studio"),
    ]);

    let key = key.replace("FMOD_", "");
    let key = key.replace("STUDIO_SYSTEM", "STUDIOSYSTEM");
    let key = key.replace("STUDIO_", "");
    let name = key.to_case(Case::Pascal);
    let name = match renames.get(&name[..]) {
        None => name,
        Some(rename) => rename.to_string(),
    };
    format_ident!("{}", name)
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

pub fn generate_struct_code(key: &String, methods: &Vec<&Function>) -> TokenStream {
    let name = format_struct_name(key);
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
                        Err(err!(#function_name, result))
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

    Ok(quote! {
        use std::ptr::null_mut;
        pub mod ffi;

        #[derive(Debug)]
        pub struct Error {
            pub function: String,
            pub code: i32,
            pub message: String
        }


        macro_rules! err {
            ($function:expr, $code:expr) => {
                Error {
                    function: $function.to_string(),
                    code: $code,
                    message: ffi::map_fmod_error($code).to_string()
                }
            };
        }

        #(#structs)*
    })
}

pub fn generate(api: &Api) -> Result<String, Error> {
    let code = generate_lib_code(api)?;
    rustfmt_wrapper::rustfmt(code).map_err(Error::from)
}
