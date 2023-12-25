use std::collections::{BTreeMap, HashSet};
use std::ops::AddAssign;
use std::str::FromStr;

use convert_case::{Case, Casing};
use quote::__private::{Ident, TokenStream};

use crate::ffi;
use crate::ffi::describe_pointer;
use crate::generators::dictionary::{KEYWORDS, RENAMES};
use crate::models::Type::{FundamentalType, UserType};
use crate::models::{
    Api, Argument, Enumeration, Error, Field, Function, Modifier, Pointer, Structure, Type,
};

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
    ("FMOD_STUDIO_LOAD_MEMORY", "FMOD_STUDIO_LOAD_MEMORY_MEMORY"),
    (
        "FMOD_STUDIO_LOAD_MEMORY_POINT",
        "FMOD_STUDIO_LOAD_MEMORY_MEMORY_POINT",
    ),
];

fn format_variant(enumeration: &str, name: &str) -> Ident {
    let name = match ENUMERATOR_RENAMES.iter().find(|pair| pair.0 == name) {
        None => name,
        Some(pair) => pair.1,
    };
    let enumeration_words: Vec<&str> = enumeration.split("_").collect();
    let variant_words: Vec<&str> = name.split("_").collect();
    // enumeration:
    // ["FMOD", "STUDIO", "PLAYBACK", "STATE"]
    // variants:
    // ["FMOD", "STUDIO", "PLAYBACK", "SUSTAINING"]
    // ["FMOD", "STUDIO", "PLAYBACK", "STOPPED"]
    // ...
    let key = variant_words
        .into_iter()
        .enumerate()
        .skip_while(|(index, word)| enumeration_words.get(*index) == Some(word))
        .map(|(_, word)| word)
        .collect::<Vec<&str>>()
        .join("_");

    let key = if key.starts_with("3D") {
        format!("{}3d", &key[2..])
    } else {
        key
    };

    let key = if key.starts_with("2D") {
        format!("{}2d", &key[2..])
    } else {
        key
    };

    let key = key.to_case(Case::UpperCamel);
    let name = key;
    let name = match RENAMES.get(&name[..]) {
        None => name,
        Some(rename) => rename.to_string(),
    };
    format_ident!("{}", name)
}

fn extract_method_name(name: &str) -> String {
    match name.rfind('_') {
        Some(index) => name[index..]
            .to_string()
            .to_case(Case::Snake)
            .replace("3_d", "3d"),
        None => name.to_string(),
    }
}

fn format_struct_ident(key: &str) -> Ident {
    let key = key.replace("FMOD_RESULT", "FMOD_FMODRESULT");
    let key = key.replace("FMOD_", "");
    let key = key.replace("STUDIO_SYSTEM", "STUDIOSYSTEM");
    let key = key.replace("STUDIO_ADVANCEDSETTINGS", "STUDIOADVANCEDSETTINGS");
    let key = key.replace("STUDIO_CPU_USAGE", "STUDIOCPUUSAGE");
    let key = key.replace("STUDIO_", "");
    let name = key.to_case(Case::Pascal);
    let name = match RENAMES.get(&name[..]) {
        None => name,
        Some(rename) => rename.to_string(),
    };
    format_ident!("{}", name)
}

pub fn format_argument_ident(name: &str) -> Ident {
    let name = name.replace("3D", "-3d-");
    let name = name.to_case(Case::Snake);
    if KEYWORDS.contains(&&*name) {
        format_ident!("{}_", name)
    } else {
        format_ident!("{}", name)
    }
}

pub fn format_rust_type(
    c_type: &Type,
    as_const: &Option<String>,
    pointer: &Option<Pointer>,
    as_array: &Option<TokenStream>,
    api: &Api,
) -> TokenStream {
    let ptr = describe_pointer(as_const, pointer);
    let tokens = match c_type {
        FundamentalType(name) => match (ptr, &name[..]) {
            ("*const", "char") => quote! { String },
            ("*const *const", "char") => quote! { Vec<String> },
            ("*mut", "char") => quote! { String },
            ("*mut", "void") => quote! { *mut c_void },
            ("*mut", "int") => quote! { Vec<i32> },
            ("*mut", "float") => quote! { Vec<f32> },
            ("*mut *mut", "float") => quote! { Vec<f32> },
            ("*mut *mut", "char") => quote! { Vec<String> },
            ("", "unsigned char") => quote! { u8 },
            ("", "char") => quote! { c_char },
            ("", "int") => quote! { i32 },
            ("", "unsigned int") => quote! { u32 },
            ("", "short") => quote! { i16 },
            ("", "unsigned short") => quote! { u16 },
            ("", "long long") => quote! { i64 },
            ("", "long") => quote! { i64 },
            ("", "unsigned long long") => quote! { u64 },
            ("", "unsigned long") => quote! { u64 },
            ("", "float") => quote! { f32 },
            _ => {
                let name = format_ident!("{}", name);
                quote! { Box<#name> }
            }
        },
        UserType(name) => match (ptr, api.describe_user_type(name)) {
            ("*mut", UserTypeDesc::OpaqueType) => {
                let name = format_struct_ident(name);
                quote! { #name }
            }
            ("*mut", UserTypeDesc::Structure) => {
                let name = format_struct_ident(name);
                quote! { #name }
            }
            ("*mut *mut", UserTypeDesc::Structure) => {
                let name = format_struct_ident(name);
                quote! { Vec<#name> }
            }
            ("*mut", UserTypeDesc::Flags) => {
                let name = format_ident!("{}", name);
                quote! { Vec<ffi::#name> }
            }
            ("*mut", UserTypeDesc::Enumeration) => {
                let name = format_struct_ident(name);
                quote! { Vec<#name> }
            }
            ("", UserTypeDesc::Structure) => {
                let name = format_struct_ident(name);
                quote! { #name }
            }
            ("", UserTypeDesc::Enumeration) => {
                let name = format_struct_ident(name);
                quote! { #name }
            }
            ("", _) => {
                let name = format_ident!("{}", name);
                quote! { ffi::#name }
            }
            _ => quote! { err },
        },
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

pub fn generate_enumeration(enumeration: &Enumeration) -> TokenStream {
    let name = format_struct_ident(&enumeration.name);

    let mut variants = vec![];
    let mut enumerator_arms = vec![];
    let mut variant_arms = vec![];

    for enumerator in &enumeration.enumerators {
        if enumerator.name.ends_with("FORCEINT") {
            continue;
        }
        let variant = format_variant(&enumeration.name, &enumerator.name);
        let enumerator = format_ident!("{}", enumerator.name);
        enumerator_arms.push(quote! {#name::#variant => ffi::#enumerator});
        variant_arms.push(quote! {ffi::#enumerator => Ok(#name::#variant)});
        variants.push(variant);
    }

    let enumeration_name = &enumeration.name;
    let enumeration = format_ident!("{}", enumeration_name);

    quote! {
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum #name {
            #(#variants),*
        }

        impl From<#name> for ffi::#enumeration {
            fn from(value: #name) -> ffi::#enumeration {
                match value {
                    #(#enumerator_arms),*
                }
            }
        }

        impl #name {
            pub fn from(value: ffi::#enumeration) -> Result<#name, Error> {
                match value {
                    #(#variant_arms),*,
                    _ => Err(err_enum!(#enumeration_name, value)),
                }
            }
        }
    }
}

pub fn generate_field(structure: &Structure, field: &Field, api: &Api) -> TokenStream {
    match (&structure.name[..], &field.name[..]) {
        ("FMOD_ADVANCEDSETTINGS", "cbSize") => {
            return quote! {};
        }
        ("FMOD_STUDIO_ADVANCEDSETTINGS", "cbsize") => {
            return quote! {};
        }
        ("FMOD_CREATESOUNDEXINFO", "cbsize") => {
            return quote! {};
        }
        ("FMOD_DSP_DESCRIPTION", "numparameters") => {
            return quote! {};
        }
        ("FMOD_DSP_PARAMETER_FFT", "spectrum") => {
            return quote! {
                pub spectrum: Vec<Vec<f32>>
            };
        }
        ("FMOD_DSP_PARAMETER_FFT", "numchannels") => {
            return quote! {};
        }
        _ => {}
    }

    let name = format_argument_ident(&field.name);
    let as_array = match &field.as_array {
        None => None,
        Some(dimension) => {
            let token = &dimension[1..dimension.len() - 1];
            let dimension = match api.describe_user_type(token) {
                UserTypeDesc::Constant => {
                    let name = format_ident!("{}", token);
                    quote! { ffi::#name }
                }
                _ => TokenStream::from_str(token).expect("not implemented yet"),
            };
            Some(dimension)
        }
    };
    let field_type = format_rust_type(
        &field.field_type,
        &field.as_const,
        &field.pointer,
        &as_array,
        &api,
    );
    quote! {
        pub #name: #field_type
    }
}

pub fn generate_field_from(structure: &str, field: &Field, api: &Api) -> TokenStream {
    let name = format_argument_ident(&field.name);
    let value_name = ffi::format_rust_ident(&field.name);
    let ptr = describe_pointer(&field.as_const, &field.pointer);

    let getter = match (structure, &field.name[..]) {
        ("FMOD_DSP_PARAMETER_3DATTRIBUTES_MULTI", "relative") => {
            quote! { attr3d_array8(value.relative.map(Attributes3d::try_from).into_iter().collect::<Result<Vec<Attributes3d>, Error>>()?) }
        }
        ("FMOD_CREATESOUNDEXINFO", "inclusionlist") => {
            quote! { to_vec!(value.inclusionlist, value.inclusionlistnum) }
        }
        ("FMOD_ADVANCEDSETTINGS", "ASIOChannelList") => {
            quote! { to_vec!(value.ASIOChannelList, value.ASIONumChannels, |ptr| to_string!(ptr))? }
        }
        ("FMOD_ADVANCEDSETTINGS", "ASIOSpeakerList") => {
            quote! { to_vec!(value.ASIOSpeakerList, value.ASIONumChannels, Speaker::from)? }
        }
        ("FMOD_OUTPUT_OBJECT3DINFO", "buffer") => {
            quote! { to_vec!(value.buffer, value.bufferlength) }
        }
        ("FMOD_DSP_BUFFER_ARRAY", "buffernumchannels") => {
            quote! { to_vec!(value.buffernumchannels, value.numbuffers) }
        }
        ("FMOD_DSP_BUFFER_ARRAY", "bufferchannelmask") => {
            quote! { to_vec!(value.bufferchannelmask, value.numbuffers) }
        }
        ("FMOD_DSP_BUFFER_ARRAY", "buffers") => {
            quote! { to_vec!(value.buffers, value.numbuffers, |ptr| Ok(*ptr))? }
        }
        ("FMOD_DSP_PARAMETER_FLOAT_MAPPING_PIECEWISE_LINEAR", "pointparamvalues") => {
            quote! { to_vec!(value.pointparamvalues, value.numpoints) }
        }
        ("FMOD_DSP_PARAMETER_FLOAT_MAPPING_PIECEWISE_LINEAR", "pointpositions") => {
            quote! { to_vec!(value.pointpositions, value.numpoints) }
        }
        ("FMOD_DSP_PARAMETER_DESC_INT", "valuenames") => {
            quote! { vec![] } // TODO
        }
        ("FMOD_DSP_PARAMETER_DESC_BOOL", "valuenames") => {
            quote! { vec![] } // TODO
        }
        ("FMOD_DSP_PARAMETER_FFT", "spectrum") => {
            quote! { to_vec!(value.spectrum.as_ptr(), value.numchannels, |ptr| Ok(to_vec!(ptr, value.length)))? }
        }
        ("FMOD_DSP_DESCRIPTION", "paramdesc") => {
            quote! { to_vec!(*value.paramdesc, value.numparameters, DspParameterDesc::try_from)? }
        }
        ("FMOD_DSP_STATE", "sidechaindata") => {
            quote! { to_vec!(value.sidechaindata, value.sidechainchannels) }
        }
        _ => match &field.field_type {
            FundamentalType(name) => match (ptr, &name[..]) {
                ("*const", "char") => quote! { to_string!(value.#value_name)? },
                ("*mut", "char") => quote! { to_string!(value.#value_name)? },
                _ => quote! { value.#value_name },
            },
            UserType(name) => match (ptr, api.describe_user_type(name)) {
                ("*mut", UserTypeDesc::OpaqueType) => {
                    let name = format_struct_ident(name);
                    quote! { #name::from(value.#value_name) }
                }
                ("*mut", UserTypeDesc::Structure) => {
                    let name = format_struct_ident(name);
                    quote! { #name::try_from(*value.#value_name)? }
                }
                ("", UserTypeDesc::Structure) => {
                    let name = format_struct_ident(name);
                    quote! { #name::try_from(value.#value_name)? }
                }
                ("", UserTypeDesc::Enumeration) => {
                    let name = format_struct_ident(name);
                    quote! { #name::from(value.#value_name)? }
                }
                _ => quote! { value.#value_name },
            },
        },
    };

    quote! {#name: #getter}
}

pub fn generate_into_field(structure: &str, field: &Field, api: &Api) -> TokenStream {
    let name = ffi::format_rust_ident(&field.name);
    let self_name = format_argument_ident(&field.name);
    let ptr = describe_pointer(&field.as_const, &field.pointer);

    let getter = match (structure, &field.name[..]) {
        ("FMOD_ADVANCEDSETTINGS", "cbSize") => {
            quote! { size_of::<ffi::FMOD_ADVANCEDSETTINGS>() as i32 }
        }
        ("FMOD_STUDIO_ADVANCEDSETTINGS", "cbsize") => {
            quote! { size_of::<ffi::FMOD_STUDIO_ADVANCEDSETTINGS>() as i32 }
        }
        ("FMOD_CREATESOUNDEXINFO", "cbsize") => {
            quote! { size_of::<ffi::FMOD_CREATESOUNDEXINFO>() as i32 }
        }
        ("FMOD_DSP_DESCRIPTION", "numparameters") => {
            quote! { self.paramdesc.len() as i32 }
        }
        ("FMOD_DSP_PARAMETER_3DATTRIBUTES_MULTI", "relative") => {
            quote! { self.relative.map(Attributes3d::into) }
        }
        ("FMOD_CREATESOUNDEXINFO", "inclusionlist") => {
            quote! { self.inclusionlist.as_ptr() as *mut _ }
        }
        ("FMOD_OUTPUT_OBJECT3DINFO", "buffer") => {
            quote! { self.buffer.as_ptr() as *mut _ }
        }
        ("FMOD_ADVANCEDSETTINGS", "ASIOChannelList") => {
            quote! { self.asio_channel_list.into_iter().map(|val| val.as_ptr()).collect::<Vec<_>>().as_mut_ptr().cast() }
        }
        ("FMOD_ADVANCEDSETTINGS", "ASIOSpeakerList") => {
            quote! { self.asio_speaker_list.into_iter().map(|val| val.into()).collect::<Vec<_>>().as_mut_ptr() }
        }
        ("FMOD_DSP_BUFFER_ARRAY", "buffernumchannels") => {
            quote! { self.buffernumchannels.as_ptr() as *mut _ }
        }
        ("FMOD_DSP_BUFFER_ARRAY", "bufferchannelmask") => {
            quote! { self.bufferchannelmask.as_ptr() as *mut _ }
        }
        ("FMOD_DSP_BUFFER_ARRAY", "buffers") => {
            quote! { self.buffers.as_ptr() as *mut _ }
        }
        ("FMOD_DSP_PARAMETER_FLOAT_MAPPING_PIECEWISE_LINEAR", "pointparamvalues") => {
            quote! { self.pointparamvalues.as_ptr() as *mut _ }
        }
        ("FMOD_DSP_PARAMETER_FLOAT_MAPPING_PIECEWISE_LINEAR", "pointpositions") => {
            quote! { self.pointpositions.as_ptr() as *mut _ }
        }
        ("FMOD_DSP_PARAMETER_DESC_INT", "valuenames") => {
            quote! { self.valuenames.as_ptr() as *mut _ }
        }
        ("FMOD_DSP_PARAMETER_DESC_BOOL", "valuenames") => {
            quote! { self.valuenames.as_ptr() as *mut _ }
        }
        ("FMOD_DSP_DESCRIPTION", "paramdesc") => {
            quote! { &mut vec_as_mut_ptr(self.paramdesc, |param| param.into()) }
        }
        ("FMOD_DSP_STATE", "sidechaindata") => {
            quote! { self.sidechaindata.as_ptr() as *mut _ }
        }
        ("FMOD_DSP_PARAMETER_FFT", "numchannels") => {
            quote! { self.spectrum.len() as i32 }
        }
        ("FMOD_DSP_PARAMETER_FFT", "spectrum") => {
            quote! { [null_mut(); 32] }
        }
        _ => match &field.field_type {
            FundamentalType(name) => match (ptr, &name[..]) {
                ("*const", "char") => quote! { self.#self_name.as_ptr().cast() },
                ("*mut", "char") => quote! { self.#self_name.as_ptr() as *mut _ },
                _ => quote! { self.#self_name },
            },
            UserType(name) => match (ptr, api.describe_user_type(name)) {
                ("*mut", UserTypeDesc::OpaqueType) => {
                    quote! { self.#self_name.as_mut_ptr() }
                }
                ("*mut", UserTypeDesc::Structure) => {
                    quote! { &mut self.#self_name.into() }
                }
                ("", UserTypeDesc::Structure) => {
                    quote! { self.#self_name.into() }
                }
                ("", UserTypeDesc::Enumeration) => {
                    quote! { self.#self_name.into() }
                }
                _ => quote! { self.#self_name },
            },
        },
    };

    quote! {#name: #getter}
}

pub fn generate_presets(structure: &Structure, api: &Api) -> TokenStream {
    let mut presets = vec![];
    if structure.name == "FMOD_REVERB_PROPERTIES" {
        for preset in &api.presets {
            let ident = format_ident!("{}", preset.name);
            let preset = preset.name.replace("FMOD_PRESET_", "").to_lowercase();
            let preset = format_ident!("{}", preset);
            let preset = quote! {
                #[inline]
                pub fn #preset() -> Self {
                    Self::try_from(ffi::#ident).unwrap()
                }
            };
            presets.push(preset);
        }
    }
    let name = format_struct_ident(&structure.name);
    if presets.is_empty() {
        quote! {}
    } else {
        quote! {
            impl #name {
                #(#presets)*
            }
        }
    }
}

pub fn generate_structure_into(structure: &Structure, api: &Api) -> TokenStream {
    let ident = format_ident!("{}", structure.name);
    let name = format_struct_ident(&structure.name);
    let conversion = structure
        .fields
        .iter()
        .map(|field| generate_into_field(&structure.name, field, api));
    let union = if structure.union.is_some() {
        Some(quote! { ,union: self.union })
    } else {
        None
    };
    quote! {
        impl Into<ffi::#ident> for #name {
            fn into(self) -> ffi::#ident {
                ffi::#ident {
                    #(#conversion),*
                    #union
                }
            }
        }
    }
}

fn is_convertable(structure: &Structure, field: &Field) -> bool {
    match (&structure.name[..], &field.name[..]) {
        ("FMOD_ADVANCEDSETTINGS", "cbSize") => false,
        ("FMOD_STUDIO_ADVANCEDSETTINGS", "cbsize") => false,
        ("FMOD_CREATESOUNDEXINFO", "cbsize") => false,
        ("FMOD_DSP_DESCRIPTION", "numparameters") => false,
        ("FMOD_DSP_PARAMETER_FFT", "numchannels") => false,
        _ => true,
    }
}

pub fn generate_structure_try_from(structure: &Structure, api: &Api) -> TokenStream {
    let ident = format_ident!("{}", structure.name);
    let name = format_struct_ident(&structure.name);
    let conversion = structure
        .fields
        .iter()
        .filter(|field| is_convertable(&structure, field))
        .map(|field| generate_field_from(&structure.name, field, api));
    let union = if structure.union.is_some() {
        Some(quote! { ,union: value.union })
    } else {
        None
    };
    let conversions = api.conversions.get(&structure.name);
    quote! {
        impl TryFrom<ffi::#ident> for #name {
            type Error = Error;

            fn try_from(value: ffi::#ident) -> Result<Self, Self::Error> {
                unsafe {
                    Ok(#name {
                        #(#conversion),*
                        #union
                    })
                }
            }
        }
        #conversions
    }
}

pub fn generate_structure(structure: &Structure, api: &Api) -> TokenStream {
    let name = format_struct_ident(&structure.name);
    let mut fields: Vec<TokenStream> = structure
        .fields
        .iter()
        .filter(|field| is_convertable(&structure, field))
        .map(|field| generate_field(structure, field, api))
        .collect();

    let mut derive = quote! { Debug, Clone };
    if structure.union.is_some() {
        let name = format_ident!("{}_UNION", structure.name);
        fields.push(quote! {
            pub union: ffi::#name
        });
        derive = quote! { Clone };
    }
    if structure.name == "FMOD_DSP_DESCRIPTION" {
        derive = quote! { Clone }
    }
    let presets = generate_presets(structure, api);
    let into = generate_structure_into(structure, api);
    let try_from = generate_structure_try_from(structure, api);
    quote! {
        #[derive(#derive)]
        pub struct #name {
            #(#fields),*
        }
        #presets
        #try_from
        #into
    }
}

struct OutArgument {
    pub target: TokenStream,
    pub source: TokenStream,
    pub output: TokenStream,
    pub retype: TokenStream,
}

struct InArgument {
    pub param: TokenStream,
    pub input: TokenStream,
}

pub fn quote_tuple(items: &Vec<TokenStream>) -> TokenStream {
    match items.len() {
        0 => quote! { () },
        1 => {
            let item = &items[0];
            quote! { #item }
        }
        _ => quote! { (#(#items),*) },
    }
}

fn map_optional(argument: &Argument, api: &Api) -> InArgument {
    let pointer = ffi::describe_pointer(&argument.as_const, &argument.pointer);
    let name = format_argument_ident(&argument.name);
    match &argument.argument_type {
        FundamentalType(type_name) => match &format!("{}:{}", pointer, type_name)[..] {
            ":int" => InArgument {
                param: quote! { #name: Option<i32> },
                input: quote! { #name.unwrap_or(0) },
            },
            ":float" => InArgument {
                param: quote! { #name: Option<f32> },
                input: quote! { #name.unwrap_or(0.0) },
            },
            ":unsigned long long" => InArgument {
                param: quote! { #name: Option<u64> },
                input: quote! { #name.unwrap_or(0) },
            },
            ":unsigned int" => InArgument {
                param: quote! { #name: Option<u32> },
                input: quote! { #name.unwrap_or(0) },
            },
            "*mut:float" => InArgument {
                param: quote! { #name: Option<*mut f32> },
                input: quote! { #name.unwrap_or(null_mut()) },
            },
            "*const:char" => InArgument {
                param: quote! { #name: Option<String> },
                input: quote! { #name.map(|value| CString::new(value).map(|value| value.as_ptr())).unwrap_or(Ok(null_mut()))? },
            },
            "*mut:void" => InArgument {
                param: quote! { #name: Option<*mut c_void> },
                input: quote! { #name.unwrap_or(null_mut()) },
            },
            argument_type => {
                unimplemented!("opt {}", argument_type)
            }
        },
        UserType(user_type) => {
            let tp = format_struct_ident(&user_type);
            let ident = format_ident!("{}", user_type);
            match (pointer, api.describe_user_type(&user_type)) {
                ("*mut", UserTypeDesc::Structure) => InArgument {
                    param: quote! { #name: Option<#tp> },
                    input: quote! { #name.map(|value| &mut value.into() as *mut _).unwrap_or(null_mut()) },
                },
                ("*mut", UserTypeDesc::OpaqueType) => InArgument {
                    param: quote! { #name: Option<#tp> },
                    input: quote! { #name.map(|value| value.as_mut_ptr()).unwrap_or(null_mut()) },
                },
                ("*const", UserTypeDesc::Structure) => InArgument {
                    param: quote! { #name: Option<#tp> },
                    input: quote! { #name.map(|value| &value.into() as *const _).unwrap_or(null()) },
                },
                ("", UserTypeDesc::Enumeration) => InArgument {
                    param: quote! { #name: Option<#tp> },
                    input: quote! { #name.map(|value| value.into()).unwrap_or(0) },
                },
                ("", UserTypeDesc::Callback) => InArgument {
                    param: quote! { #name: ffi::#ident },
                    input: quote! { #name },
                },
                user_type => unimplemented!("opt {:?}", user_type),
            }
        }
    }
}

fn map_input(argument: &Argument, api: &Api) -> InArgument {
    let pointer = ffi::describe_pointer(&argument.as_const, &argument.pointer);
    let argument_type = &argument.argument_type;
    let argument = format_argument_ident(&argument.name);
    match argument_type {
        FundamentalType(type_name) => match &format!("{}:{}", pointer, type_name)[..] {
            ":float" => InArgument {
                param: quote! { #argument: f32 },
                input: quote! { #argument },
            },
            ":int" => InArgument {
                param: quote! { #argument: i32 },
                input: quote! { #argument },
            },
            ":unsigned int" => InArgument {
                param: quote! { #argument: u32 },
                input: quote! { #argument },
            },
            ":unsigned long long" => InArgument {
                param: quote! { #argument: u64 },
                input: quote! { #argument },
            },
            "*const:char" => InArgument {
                param: quote! { #argument: &str },
                input: quote! { CString::new(#argument)?.as_ptr() },
            },
            "*mut:void" => InArgument {
                param: quote! { #argument: *mut c_void },
                input: quote! { #argument },
            },
            "*const:void" => InArgument {
                param: quote! { #argument: *const c_void },
                input: quote! { #argument },
            },
            "*mut:float" => InArgument {
                param: quote! { #argument: *mut f32 },
                input: quote! { #argument },
            },
            _ => unimplemented!(),
        },
        UserType(type_name) => {
            let rust_type = format_struct_ident(&type_name);
            let ident = format_ident!("{}", type_name);
            match (pointer, api.describe_user_type(&type_name)) {
                ("*mut", UserTypeDesc::OpaqueType) => InArgument {
                    param: quote! { #argument: #rust_type },
                    input: quote! { #argument.as_mut_ptr() },
                },
                ("*const", UserTypeDesc::Structure) => InArgument {
                    param: quote! { #argument: #rust_type },
                    input: quote! { &#argument.into() },
                },
                ("*mut", UserTypeDesc::Structure) => InArgument {
                    param: quote! { #argument: #rust_type },
                    input: quote! { &mut #argument.into() },
                },
                ("", UserTypeDesc::Structure) => InArgument {
                    param: quote! { #argument: #rust_type },
                    input: quote! { #argument.into() },
                },
                ("", UserTypeDesc::Flags) => InArgument {
                    param: quote! { #argument: impl Into<ffi::#ident> },
                    input: quote! { #argument.into() },
                },
                ("", UserTypeDesc::Enumeration) => InArgument {
                    param: quote! { #argument: #rust_type },
                    input: quote! { #argument.into() },
                },
                ("", UserTypeDesc::Callback) => InArgument {
                    param: quote! { #argument: ffi::#ident },
                    input: quote! { #argument },
                },
                ("", UserTypeDesc::TypeAlias) => match &type_name[..] {
                    "FMOD_BOOL" => InArgument {
                        param: quote! { #argument: bool },
                        input: quote! { from_bool!(#argument) },
                    },
                    "FMOD_PORT_INDEX" => InArgument {
                        param: quote! { #argument: u64 },
                        input: quote! { #argument },
                    },
                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            }
        }
    }
}

fn map_output(argument: &Argument, _function: &Function, api: &Api) -> OutArgument {
    let pointer = ffi::describe_pointer(&argument.as_const, &argument.pointer);
    let arg = format_argument_ident(&argument.name);

    match &argument.argument_type {
        FundamentalType(type_name) => match &format!("{}:{}", pointer, type_name)[..] {
            "*mut:char" => OutArgument {
                target: quote! { let #arg = CString::from_vec_unchecked(b"".to_vec()).into_raw(); },
                source: quote! { #arg },
                output: quote! { CString::from_raw(#arg).into_string().map_err(Error::String)? },
                retype: quote! { String },
            },
            "*mut:float" => OutArgument {
                target: quote! { let mut #arg = f32::default(); },
                source: quote! { &mut #arg },
                output: quote! { #arg },
                retype: quote! { f32 },
            },
            "*mut:unsigned long long" => OutArgument {
                target: quote! { let mut #arg = u64::default(); },
                source: quote! { &mut #arg },
                output: quote! { #arg },
                retype: quote! { u64 },
            },
            "*mut:long long" => OutArgument {
                target: quote! { let mut #arg = i64::default(); },
                source: quote! { &mut #arg },
                output: quote! { #arg },
                retype: quote! { i64 },
            },
            "*mut:unsigned int" => OutArgument {
                target: quote! { let mut #arg = u32::default(); },
                source: quote! { &mut #arg },
                output: quote! { #arg },
                retype: quote! { u32 },
            },
            "*mut:int" => OutArgument {
                target: quote! { let mut #arg = i32::default(); },
                source: quote! { &mut #arg },
                output: quote! { #arg },
                retype: quote! { i32 },
            },
            "*mut *mut:void" => OutArgument {
                target: quote! { let mut #arg = null_mut(); },
                source: quote! { &mut #arg },
                output: quote! { #arg },
                retype: quote! { *mut c_void },
            },
            "*mut:void" => OutArgument {
                target: quote! { let #arg = null_mut(); },
                source: quote! { #arg },
                output: quote! { #arg },
                retype: quote! { *mut c_void },
            },
            _ => unimplemented!(),
        },
        UserType(user_type) => {
            let type_name = format_struct_ident(&user_type);
            let ident = format_ident!("{}", user_type);

            match (pointer, api.describe_user_type(&user_type)) {
                ("*mut", UserTypeDesc::TypeAlias) => match &user_type[..] {
                    "FMOD_BOOL" => OutArgument {
                        target: quote! { let mut #arg = ffi::FMOD_BOOL::default(); },
                        source: quote! { &mut #arg },
                        output: quote! { to_bool!(#arg) },
                        retype: quote! { bool },
                    },
                    "FMOD_PORT_INDEX" => OutArgument {
                        target: quote! { let mut #arg = u64::default(); },
                        source: quote! { &mut #arg },
                        output: quote! { #arg },
                        retype: quote! { u64 },
                    },
                    _ => unimplemented!(),
                },
                ("*mut *mut", UserTypeDesc::OpaqueType) => OutArgument {
                    target: quote! { let mut #arg = null_mut(); },
                    source: quote! { &mut #arg },
                    output: quote! { #type_name::from(#arg) },
                    retype: quote! { #type_name },
                },
                ("*mut", UserTypeDesc::Flags) => OutArgument {
                    target: quote! { let mut #arg = ffi::#ident::default(); },
                    source: quote! { &mut #arg },
                    output: quote! { #arg },
                    retype: quote! { ffi::#ident },
                },
                ("*mut", UserTypeDesc::Structure) => OutArgument {
                    target: quote! { let mut #arg = ffi::#ident::default(); },
                    source: quote! { &mut #arg },
                    output: quote! { #type_name::try_from(#arg)? },
                    retype: quote! { #type_name },
                },
                ("*mut *mut", UserTypeDesc::Structure) => OutArgument {
                    target: quote! { let mut #arg = null_mut(); },
                    source: quote! { &mut #arg },
                    output: quote! { #type_name::try_from(*#arg)? },
                    retype: quote! { #type_name },
                },
                ("*const *const", UserTypeDesc::Structure) => OutArgument {
                    target: quote! { let mut #arg = null(); },
                    source: quote! { &mut #arg },
                    output: quote! { #type_name::try_from(*#arg)? },
                    retype: quote! { #type_name },
                },
                ("*mut", UserTypeDesc::Enumeration) => OutArgument {
                    target: quote! { let mut #arg = ffi::#ident::default(); },
                    source: quote! { &mut #arg },
                    output: quote! { #type_name::from(#arg)? },
                    retype: quote! { #type_name },
                },
                _ => unimplemented!(),
            }
        }
    }
}

struct Signature {
    pub arguments: Vec<TokenStream>,
    pub inputs: Vec<TokenStream>,
    pub targets: Vec<TokenStream>,
    pub outputs: Vec<TokenStream>,
    pub return_types: Vec<TokenStream>,
}

impl Signature {
    pub fn new() -> Self {
        Self {
            arguments: vec![],
            inputs: vec![],
            targets: vec![],
            outputs: vec![],
            return_types: vec![],
        }
    }

    pub fn define(
        self,
    ) -> (
        Vec<TokenStream>,
        Vec<TokenStream>,
        Vec<TokenStream>,
        TokenStream,
        TokenStream,
    ) {
        (
            self.arguments,
            self.inputs,
            self.targets,
            quote_tuple(&self.outputs),
            quote_tuple(&self.return_types),
        )
    }

    pub fn overwrites(&mut self, owner: &str, function: &Function, argument: &Argument) -> bool {
        let pointer = ffi::describe_pointer(&argument.as_const, &argument.pointer);
        if self.arguments.is_empty()
            && argument.argument_type.is_user_type(owner)
            && pointer == "*mut"
        {
            self.arguments.push(quote! { &self });
            self.inputs.push(quote! { self.pointer });
            return true;
        }

        if function.name == "FMOD_Studio_System_Create" && argument.name == "headerversion" {
            self.inputs.push(quote! { ffi::FMOD_VERSION });
            return true;
        }

        if function.name == "FMOD_System_Create" && argument.name == "headerversion" {
            self.inputs.push(quote! { ffi::FMOD_VERSION });
            return true;
        }

        // FMOD_Sound_Set3DCustomRolloff
        if function.name == "FMOD_Sound_Set3DCustomRolloff" && argument.name == "numpoints" {
            self.targets
                .push(quote! { let numpoints = points.len() as i32; });
            self.inputs.push(quote! { numpoints });
            return true;
        }
        if function.name == "FMOD_Sound_Set3DCustomRolloff" && argument.name == "points" {
            self.arguments.push(quote! { points: Vec<Vector> });
            self.inputs
                .push(quote! { vec_as_mut_ptr(points, |point| point.into()) });
            return true;
        }
        if function.name == "FMOD_Sound_Get3DCustomRolloff" && argument.name == "numpoints" {
            self.targets
                .push(quote! { let mut numpoints = i32::default(); });
            self.inputs.push(quote! { &mut numpoints });
            return true;
        }
        if function.name == "FMOD_Sound_Get3DCustomRolloff" && argument.name == "points" {
            self.targets.push(quote! { let mut points = null_mut(); });
            self.inputs.push(quote! { &mut points });
            self.outputs
                .push(quote! { to_vec!(points, numpoints, Vector::try_from)? });
            self.return_types.push(quote! { Vec<Vector> });
            return true;
        }

        // FMOD_Channel_Set3DCustomRolloff
        if function.name == "FMOD_Channel_Set3DCustomRolloff" && argument.name == "numpoints" {
            self.targets
                .push(quote! { let numpoints = points.len() as i32; });
            self.inputs.push(quote! { numpoints });
            return true;
        }
        if function.name == "FMOD_Channel_Set3DCustomRolloff" && argument.name == "points" {
            self.arguments.push(quote! { points: Vec<Vector> });
            self.inputs
                .push(quote! { vec_as_mut_ptr(points, |point| point.into()) });
            return true;
        }
        if function.name == "FMOD_Channel_Get3DCustomRolloff" && argument.name == "numpoints" {
            self.targets
                .push(quote! { let mut numpoints = i32::default(); });
            self.inputs.push(quote! { &mut numpoints });
            return true;
        }
        if function.name == "FMOD_Channel_Get3DCustomRolloff" && argument.name == "points" {
            self.targets.push(quote! { let mut points = null_mut(); });
            self.inputs.push(quote! { &mut points });
            self.outputs
                .push(quote! { to_vec!(points, numpoints, Vector::try_from)? });
            self.return_types.push(quote! { Vec<Vector> });
            return true;
        }

        if function.name == "FMOD_ChannelGroup_Set3DCustomRolloff" && argument.name == "numpoints" {
            self.targets
                .push(quote! { let numpoints = points.len() as i32; });
            self.inputs.push(quote! { numpoints });
            return true;
        }
        if function.name == "FMOD_ChannelGroup_Set3DCustomRolloff" && argument.name == "points" {
            self.arguments.push(quote! { points: Vec<Vector> });
            self.inputs
                .push(quote! { vec_as_mut_ptr(points, |point| point.into()) });
            return true;
        }
        if function.name == "FMOD_ChannelGroup_Get3DCustomRolloff" && argument.name == "numpoints" {
            self.targets
                .push(quote! { let mut numpoints = i32::default(); });
            self.inputs.push(quote! { &mut numpoints });
            return true;
        }
        if function.name == "FMOD_ChannelGroup_Get3DCustomRolloff" && argument.name == "points" {
            self.targets.push(quote! { let mut points = null_mut(); });
            self.inputs.push(quote! { &mut points });
            self.outputs
                .push(quote! { to_vec!(points, numpoints, Vector::try_from)? });
            self.return_types.push(quote! { Vec<Vector> });
            return true;
        }

        if function.name == "FMOD_Studio_Bank_GetEventList" && argument.name == "count" {
            self.targets
                .push(quote! { let mut count = i32::default(); });
            self.inputs.push(quote! { &mut count });
            return true;
        }
        if function.name == "FMOD_Studio_Bank_GetEventList" && argument.name == "array" {
            self.targets
                .push(quote! { let mut array = vec![null_mut(); capacity as usize]; });
            self.inputs.push(quote! { array.as_mut_ptr() });
            self.outputs
                .push(quote! { array.into_iter().take(count as usize).map(EventDescription::from).collect() });
            self.return_types.push(quote! { Vec<EventDescription> });
            return true;
        }

        if function.name == "FMOD_Studio_Bank_GetBusList" && argument.name == "count" {
            self.targets
                .push(quote! { let mut count = i32::default(); });
            self.inputs.push(quote! { &mut count });
            return true;
        }
        if function.name == "FMOD_Studio_Bank_GetBusList" && argument.name == "array" {
            self.targets
                .push(quote! { let mut array = vec![null_mut(); capacity as usize]; });
            self.inputs.push(quote! { array.as_mut_ptr() });
            self.outputs
                .push(quote! { array.into_iter().take(count as usize).map(Bus::from).collect() });
            self.return_types.push(quote! { Vec<Bus> });
            return true;
        }

        if function.name == "FMOD_Studio_Bank_GetVCAList" && argument.name == "count" {
            self.targets
                .push(quote! { let mut count = i32::default(); });
            self.inputs.push(quote! { &mut count });
            return true;
        }
        if function.name == "FMOD_Studio_Bank_GetVCAList" && argument.name == "array" {
            self.targets
                .push(quote! { let mut array = vec![null_mut(); capacity as usize]; });
            self.inputs.push(quote! { array.as_mut_ptr() });
            self.outputs
                .push(quote! { array.into_iter().take(count as usize).map(Vca::from).collect() });
            self.return_types.push(quote! { Vec<Vca> });
            return true;
        }

        if function.name == "FMOD_Studio_EventDescription_GetInstanceList"
            && argument.name == "count"
        {
            self.targets
                .push(quote! { let mut count = i32::default(); });
            self.inputs.push(quote! { &mut count });
            return true;
        }
        if function.name == "FMOD_Studio_EventDescription_GetInstanceList"
            && argument.name == "array"
        {
            self.targets
                .push(quote! { let mut array = vec![null_mut(); capacity as usize]; });
            self.inputs.push(quote! { array.as_mut_ptr() });
            self.outputs.push(quote! { array.into_iter().take(count as usize).map(EventInstance::from).collect() });
            self.return_types.push(quote! { Vec<EventInstance> });
            return true;
        }

        if function.name == "FMOD_Studio_System_GetBankList" && argument.name == "count" {
            self.targets
                .push(quote! { let mut count = i32::default(); });
            self.inputs.push(quote! { &mut count });
            return true;
        }
        if function.name == "FMOD_Studio_System_GetBankList" && argument.name == "array" {
            self.targets
                .push(quote! { let mut array = vec![null_mut(); capacity as usize]; });
            self.inputs.push(quote! { array.as_mut_ptr() });
            self.outputs
                .push(quote! { array.into_iter().take(count as usize).map(Bank::from).collect() });
            self.return_types.push(quote! { Vec<Bank> });
            return true;
        }

        if function.name == "FMOD_Studio_System_GetParameterDescriptionList"
            && argument.name == "count"
        {
            self.targets
                .push(quote! { let mut count = i32::default(); });
            self.inputs.push(quote! { &mut count });
            return true;
        }
        if function.name == "FMOD_Studio_System_GetParameterDescriptionList"
            && argument.name == "array"
        {
            self.targets
                .push(quote! { let mut array = vec![ffi::FMOD_STUDIO_PARAMETER_DESCRIPTION::default(); capacity as usize]; });
            self.inputs.push(quote! { array.as_mut_ptr() });
            self.outputs
                .push(quote! { array.into_iter().take(count as usize).map(ParameterDescription::try_from).collect::<Result<_, Error>>()? });
            self.return_types.push(quote! { Vec<ParameterDescription> });
            return true;
        }

        return false;
    }
}

impl AddAssign<InArgument> for Signature {
    fn add_assign(&mut self, argument: InArgument) {
        self.arguments.push(argument.param);
        self.inputs.push(argument.input);
    }
}

impl AddAssign<OutArgument> for Signature {
    fn add_assign(&mut self, argument: OutArgument) {
        self.targets.push(argument.target);
        self.inputs.push(argument.source);
        self.outputs.push(argument.output);
        self.return_types.push(argument.retype);
    }
}

pub fn generate_method(owner: &str, function: &Function, api: &Api) -> TokenStream {
    let mut signature = Signature::new();

    if let Some(overriding) = api.overriding.get(&function.name) {
        return overriding.clone();
    }

    for argument in &function.arguments {
        if !signature.overwrites(owner, function, argument) {
            match api.get_modifier(&function.name, &argument.name) {
                Modifier::None => signature += map_input(argument, api),
                Modifier::Opt => signature += map_optional(argument, api),
                Modifier::Out => signature += map_output(argument, function, api),
            }
        }
    }

    let (arguments, inputs, out, output, returns) = signature.define();
    let method_name = extract_method_name(&function.name);
    let method = format_ident!("{}", method_name);
    let function_name = &function.name;
    let function = format_ident!("{}", function_name);

    quote! {
        pub fn #method( #(#arguments),* ) -> Result<#returns, Error> {
            unsafe {
                #(#out)*
                match ffi::#function( #(#inputs),* ) {
                    ffi::FMOD_OK => Ok(#output),
                    error => Err(err_fmod!(#function_name, error)),
                }
            }
        }
    }
}

pub fn generate_opaque_type(key: &String, methods: &Vec<&Function>, api: &Api) -> TokenStream {
    let name = format_struct_ident(key);
    let opaque_type = format_ident!("{}", key);

    let methods: Vec<TokenStream> = methods
        .iter()
        .map(|method| generate_method(key, method, api))
        .collect();

    quote! {
        #[derive(Debug, Clone, Copy)]
        pub struct #name {
            pointer: *mut ffi::#opaque_type,
        }

        unsafe impl Send for #name {}

        unsafe impl Sync for #name {}

        impl #name {
            #[inline]
            pub fn from(pointer: *mut ffi::#opaque_type) -> Self {
                Self { pointer }
            }
            #[inline]
            pub fn as_mut_ptr(&self) -> *mut ffi::#opaque_type {
                self.pointer
            }
            #(#methods)*
        }
    }
}

#[derive(Debug)]
enum UserTypeDesc {
    OpaqueType,
    Structure,
    Enumeration,
    Flags,
    Constant,
    TypeAlias,
    Callback,
    Unknown,
}

impl Api {
    pub fn is_structure(&self, key: &str) -> bool {
        self.structures
            .iter()
            .any(|structure| &structure.name == key)
    }

    pub fn is_opaque_type(&self, key: &str) -> bool {
        self.opaque_types
            .iter()
            .any(|opaque_type| &opaque_type.name == key)
    }

    pub fn is_enumeration(&self, key: &str) -> bool {
        self.enumerations
            .iter()
            .any(|enumeration| &enumeration.name == key)
    }

    pub fn is_flags(&self, key: &str) -> bool {
        self.flags.iter().any(|flags| &flags.name == key)
    }

    pub fn is_constant(&self, key: &str) -> bool {
        self.constants.iter().any(|constant| &constant.name == key)
    }

    pub fn is_type_alias(&self, key: &str) -> bool {
        self.type_aliases
            .iter()
            .any(|type_alias| &type_alias.name == key)
    }

    pub fn is_callback(&self, key: &str) -> bool {
        self.callbacks.iter().any(|callback| &callback.name == key)
    }

    fn describe_user_type(&self, key: &str) -> UserTypeDesc {
        if self.is_structure(key) {
            UserTypeDesc::Structure
        } else if self.is_enumeration(key) {
            UserTypeDesc::Enumeration
        } else if self.is_flags(key) {
            UserTypeDesc::Flags
        } else if self.is_opaque_type(key) {
            UserTypeDesc::OpaqueType
        } else if self.is_constant(key) {
            UserTypeDesc::Constant
        } else if self.is_type_alias(key) {
            UserTypeDesc::TypeAlias
        } else if self.is_callback(key) {
            UserTypeDesc::Callback
        } else {
            UserTypeDesc::Unknown
        }
    }

    pub fn get_modifier(&self, function: &str, argument: &str) -> Modifier {
        let key = format!("{}+{}", function, argument);
        match self.modifiers.get(&key) {
            None => Modifier::None,
            Some(modifier) => modifier.clone(),
        }
    }
}

impl Type {
    pub fn is_user_type(&self, name: &str) -> bool {
        match self {
            FundamentalType(_) => false,
            UserType(user_type) => user_type == name,
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

    let mut types: BTreeMap<String, Vec<&Function>> = BTreeMap::new();
    for ot in &opaque_types {
        types.insert(ot.clone(), vec![]);
    }
    for function in &functions {
        let key = extract_struct_key(&function.name);
        if opaque_types.contains(&key) {
            match types.get_mut(&key) {
                Some(methods) => methods.push(function),
                None => {
                    types.insert(key, vec![function]);
                }
            }
        } else {
            println!("Global function: {}", function.name);
        }
    }

    let types: Vec<TokenStream> = types
        .iter()
        .map(|(key, methods)| generate_opaque_type(key, methods, api))
        .collect();

    let enumerations: Vec<TokenStream> =
        api.enumerations.iter().map(generate_enumeration).collect();

    let mut structures: Vec<TokenStream> = vec![];
    for structure in &api.structures {
        structures.push(generate_structure(structure, api));
    }

    Ok(quote! {
        #![allow(unused_unsafe)]
        use std::os::raw::{c_char};
        use std::ffi::{c_void, CStr, CString, IntoStringError, NulError};
        use std::fmt::{Display, Formatter};
        use std::mem::size_of;
        use std::ptr::{null, null_mut};
        use std::slice;
        pub mod ffi;
        #[cfg(feature = "flags")]
        mod flags;
        #[cfg(feature = "flags")]
        pub use flags::*;

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
            },
            String(IntoStringError),
            StringNul(NulError),
            NotDspFft
        }

        impl Display for Error {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match self {
                    Error::Fmod {
                        function,
                        code,
                        message,
                    } => {
                        write!(f, "{}: {} ({})", function, message, code)
                    }
                    Error::EnumBindgen { enumeration, value } => {
                        write!(f, "FMOD returns unexpected value {} for {} enum", value, enumeration)
                    }
                    Error::String(_) => {
                        write!(f, "invalid UTF-8 when converting C string")
                    }
                    Error::StringNul(_) => {
                        write!(f, "nul byte was found in the middle, C strings can't contain it")
                    }
                    Error::NotDspFft => {
                        write!(f, "trying get FFT from DSP which not FFT")
                    }
                }
            }
        }

        impl std::error::Error for Error {}

        impl From<NulError> for Error {
            fn from(error: NulError) -> Self {
                Error::StringNul(error)
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

        macro_rules! to_string {
            ($ ptr : expr) => {
                if $ptr.is_null() {
                    Ok(String::new())
                } else {
                    CString::from(CStr::from_ptr($ptr))
                        .into_string()
                        .map_err(Error::String)
                }
            };
        }

        macro_rules! to_vec {
            ($ ptr : expr , $ length : expr, $ closure : expr) => {
                slice::from_raw_parts($ptr, $length as usize).to_vec().into_iter().map($closure).collect::<Result<Vec<_>, Error>>()
            };
            ($ ptr : expr , $ length : expr) => {
                slice::from_raw_parts($ptr, $length as usize).to_vec()
            };
        }

        macro_rules! to_bool {
            ($ value: expr ) => {
                match $value {
                    1 => true,
                    _ => false
                }
            }
        }
        macro_rules! from_bool {
            ($ value: expr ) => {
                match $value {
                    true => 1,
                    _ => 0
                }
            }
        }

        pub fn attr3d_array8(values: Vec<Attributes3d>) -> [Attributes3d; ffi::FMOD_MAX_LISTENERS as usize] {
            values.try_into().expect("slice with incorrect length")
        }

        pub fn vec_as_mut_ptr<T, O, F>(values: Vec<T>, map: F) -> *mut O
            where F: FnMut(T) -> O
        {
            let mut values = values
                .into_iter()
                .map(map)
                .collect::<Vec<O>>();
            let pointer = values.as_mut_ptr();
            std::mem::forget(values);
            pointer
        }

        #(#enumerations)*
        #(#structures)*
        #(#types)*
    })
}

pub fn generate(api: &Api) -> Result<String, Error> {
    let code = generate_lib_code(api)?;
    rustfmt_wrapper::rustfmt(code).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use crate::lib::{format_variant, generate_enumeration, generate_method, generate_structure};
    use crate::models::Type::{FundamentalType, UserType};
    use crate::models::{Argument, Enumeration, Enumerator, Field, Function, Pointer, Structure};
    use crate::Api;

    fn normal() -> Option<Pointer> {
        Some(Pointer::NormalPointer("*".into()))
    }

    #[test]
    fn test_variant_name_starts_with_same_letter_as_enumeration_name() {
        let ident = format_variant(
            "FMOD_STUDIO_PLAYBACK_STATE",
            "FMOD_STUDIO_PLAYBACK_SUSTAINING",
        );
        assert_eq!(ident, format_ident!("Sustaining"));
    }

    #[test]
    fn test_variant_name_duplicates_one_word_of_enumeration_name() {
        let ident = format_variant(
            "FMOD_STUDIO_LOADING_STATE",
            "FMOD_STUDIO_LOADING_STATE_LOADING",
        );
        assert_eq!(ident, format_ident!("Loading"));
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
        let actual = generate_method("FMOD_SYSTEM", &function, &Api::default()).to_string();
        let expected = quote! {
            pub fn set_dsp_buffer_size(&self, bufferlength: u32, numbuffers: i32) -> Result<(), Error> {
                unsafe {
                    match ffi::FMOD_System_SetDSPBufferSize(self.pointer, bufferlength, numbuffers) {
                        ffi::FMOD_OK => Ok(()),
                        error => Err(err_fmod!("FMOD_System_SetDSPBufferSize", error)),
                    }
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
        let actual = generate_enumeration(&enumeration).to_string();
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
        let actual = generate_enumeration(&enumeration).to_string();
        let expected = quote! {
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub enum SpeakerMode {
                Default,
                Mode5Point1
            }

            impl From<SpeakerMode> for ffi::FMOD_SPEAKERMODE {
                fn from(value: SpeakerMode) -> ffi::FMOD_SPEAKERMODE {
                    match value {
                        SpeakerMode::Default => ffi::FMOD_SPEAKERMODE_DEFAULT,
                        SpeakerMode::Mode5Point1 => ffi::FMOD_SPEAKERMODE_5POINT1
                    }
                }
            }

            impl SpeakerMode {
                pub fn from(value: ffi::FMOD_SPEAKERMODE) -> Result<SpeakerMode, Error> {
                    match value {
                        ffi::FMOD_SPEAKERMODE_DEFAULT => Ok(SpeakerMode::Default),
                        ffi::FMOD_SPEAKERMODE_5POINT1 => Ok(SpeakerMode::Mode5Point1),
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
        let actual = generate_enumeration(&enumeration).to_string();
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
        let actual = generate_enumeration(&enumeration).to_string();
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

    #[test]
    fn test_should_generate_structure() {
        let structure = Structure {
            name: "FMOD_VECTOR".to_string(),
            fields: vec![
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("float".to_string()),
                    pointer: None,
                    name: "x".to_string(),
                },
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("float".to_string()),
                    pointer: None,
                    name: "y".to_string(),
                },
                Field {
                    as_const: None,
                    as_array: None,
                    field_type: FundamentalType("float".to_string()),
                    pointer: None,
                    name: "z".to_string(),
                },
            ],
            union: None,
        };
        let actual = generate_structure(&structure, &Api::default()).to_string();
        let expected = quote! {
            #[derive(Debug, Clone)]
            pub struct Vector {
                pub x: f32,
                pub y: f32,
                pub z: f32
            }

            impl TryFrom<ffi::FMOD_VECTOR> for Vector {
                type Error = Error;
                fn try_from(value: ffi::FMOD_VECTOR) -> Result<Self, Self::Error> {
                    unsafe {
                        Ok(Vector {
                            x: value.x,
                            y: value.y,
                            z: value.z
                        })
                    }
                }
            }

            impl Into<ffi::FMOD_VECTOR> for Vector {
                fn into(self) -> ffi::FMOD_VECTOR {
                    ffi::FMOD_VECTOR {
                        x: self.x,
                        y: self.y,
                        z: self.z
                    }
                }
            }
        }
        .to_string();
        assert_eq!(actual, expected)
    }

    #[test]
    fn test_should_generate_structure_with_keyword_field() {
        let structure = Structure {
            name: "FMOD_PLUGINLIST".to_string(),
            fields: vec![Field {
                as_const: None,
                as_array: None,
                field_type: UserType("FMOD_PLUGINTYPE".to_string()),
                pointer: None,
                name: "type".to_string(),
            }],
            union: None,
        };
        let actual = generate_structure(&structure, &Api::default()).to_string();
        let expected = quote! {
            #[derive(Debug, Clone)]
            pub struct PluginList {
                pub type_: ffi::FMOD_PLUGINTYPE
            }

            impl TryFrom<ffi::FMOD_PLUGINLIST> for PluginList {
                type Error = Error;
                fn try_from(value: ffi::FMOD_PLUGINLIST) -> Result<Self, Self::Error> {
                    unsafe {
                        Ok(PluginList {
                            type_: value.type_
                        })
                    }
                }
            }

            impl Into<ffi::FMOD_PLUGINLIST> for PluginList {
                fn into(self) -> ffi::FMOD_PLUGINLIST {
                    ffi::FMOD_PLUGINLIST {
                        type_: self.type_
                    }
                }
            }
        }
        .to_string();
        assert_eq!(actual, expected)
    }
}
