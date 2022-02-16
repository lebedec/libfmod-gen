#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate quote;

extern crate proc_macro;

#[macro_use]
extern crate pest_derive;

use crate::generators::{ffi, lib};
use crate::models::{Api, Error, OpaqueType};
use crate::parsers::{
    fmod, fmod_codec, fmod_common, fmod_dsp, fmod_dsp_effects, fmod_errors, fmod_output,
    fmod_studio, fmod_studio_common,
};
use std::fs;
use std::path::Path;

mod generators;
mod models;
mod parsers;
mod repr;

fn generate_lib_fmod(source: &str) -> Result<(), Error> {
    let source = Path::new(source);
    let mut api = Api::default();

    let data = fs::read_to_string(source.join("api/studio/inc/fmod_studio.h"))?;
    let header = fmod_studio::parse(&data)?;
    let link = "fmodstudio".into();
    api.functions.push((link, header.functions.clone()));

    let data = fs::read_to_string(source.join("api/studio/inc/fmod_studio_common.h"))?;
    let header = fmod_studio_common::parse(&data)?;
    api.opaque_types.extend(header.opaque_types);
    api.constants.extend(header.constants);
    api.enumerations.extend(header.enumerations);
    api.callbacks.extend(header.callbacks);
    api.flags.extend(header.flags);
    api.structures.extend(header.structures);

    let data = fs::read_to_string(source.join("api/core/inc/fmod.h"))?;
    let header = fmod::parse(&data)?;
    let link = "fmod".into();
    api.functions.push((link, header.functions.clone()));

    let data = fs::read_to_string(source.join("api/core/inc/fmod_common.h"))?;
    let header = fmod_common::parse(&data)?;
    api.opaque_types.extend(header.opaque_types);
    api.type_aliases.extend(header.type_aliases);
    api.constants.extend(header.constants);
    api.enumerations.extend(header.enumerations);
    api.callbacks.extend(header.callbacks);
    api.flags.extend(header.flags);
    api.structures.extend(header.structures);
    api.presets.extend(header.presets);

    let data = fs::read_to_string(source.join("api/core/inc/fmod_codec.h"))?;
    let header = fmod_codec::parse(&data)?;
    api.opaque_types.extend(header.opaque_types);
    api.constants.extend(header.constants);
    api.callbacks.extend(header.callbacks);
    api.flags.extend(header.flags);
    api.structures.extend(header.structures);

    let data = fs::read_to_string(source.join("api/core/inc/fmod_output.h"))?;
    let header = fmod_output::parse(&data)?;
    api.opaque_types.extend(header.opaque_types);
    api.constants.extend(header.constants);
    api.callbacks.extend(header.callbacks);
    api.flags.extend(header.flags);
    api.structures.extend(header.structures);

    let data = fs::read_to_string(source.join("api/core/inc/fmod_dsp.h"))?;
    let header = fmod_dsp::parse(&data)?;
    api.opaque_types.extend(header.opaque_types);
    api.constants.extend(header.constants);
    api.enumerations.extend(header.enumerations);
    api.callbacks.extend(header.callbacks);
    api.flags.extend(header.flags);
    api.structures.extend(header.structures);

    let data = fs::read_to_string(source.join("api/core/inc/fmod_dsp_effects.h"))?;
    let header = fmod_dsp_effects::parse(&data)?;
    api.constants.extend(header.constants);
    api.enumerations.extend(header.enumerations);
    api.structures.extend(header.structures);

    let data = fs::read_to_string(source.join("api/core/inc/fmod_errors.h"))?;
    let header = fmod_errors::parse(&data)?;
    api.errors = header.mapping.clone();

    // post processing
    api.opaque_types.push(OpaqueType {
        name: "FMOD_STUDIO_SYSTEM".into(),
    });

    println!("FMOD API");
    println!("Opaque Types: {}", api.opaque_types.len());
    println!("Type Aliases: {}", api.type_aliases.len());
    println!("Structures: {}", api.structures.len());
    println!("Constants: {}", api.constants.len());
    println!("Flags: {}", api.flags.len());
    println!("Enumerations: {}", api.enumerations.len());
    println!("Callbacks: {}", api.callbacks.len());
    println!("Functions: {}", api.functions.len());
    println!("Errors: {}", api.errors.errors.len());

    let code = ffi::generate(&api)?;
    fs::write("../libfmod/src/ffi.rs", code)?;
    let code = lib::generate(&api)?;
    fs::write("../libfmod/src/lib.rs", code)?;

    Ok(())
}

fn main() {
    if let Err(error) = generate_lib_fmod("./fmod") {
        println!("Unable to generate libfmod, {:?}", error);
    }
}
