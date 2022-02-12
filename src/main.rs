#[macro_use]
extern crate quote;

extern crate proc_macro;

#[macro_use]
extern crate pest_derive;

use crate::generators::ffi;
use crate::models::OpaqueType;
use crate::parsers::{
    fmod, fmod_codec, fmod_common, fmod_dsp, fmod_dsp_effects, fmod_errors, fmod_output,
    fmod_studio, fmod_studio_common,
};
use std::fs;
use std::path::Path;

mod example;
mod generators;
mod models;
mod parsers;
mod repr;

fn generate_lib_fmod(source: &str) {
    let source = Path::new(source);

    let mut api = ffi::Api::default();

    let data =
        fs::read_to_string(source.join("api/studio/inc/fmod_studio.h")).expect("cannot read file");
    let header = fmod_studio::parse(&data).unwrap();

    let data = fs::read_to_string(source.join("api/studio/inc/fmod_studio_common.h"))
        .expect("cannot read file");
    let header = fmod_studio_common::parse(&data).unwrap();
    api.opaque_types.extend(header.opaque_types);
    api.constants.extend(header.constants);
    api.enumerations.extend(header.enumerations);
    api.callbacks.extend(header.callbacks);
    api.flags.extend(header.flags);

    let data = fs::read_to_string(source.join("api/core/inc/fmod.h")).expect("cannot read file");
    let header = fmod::parse(&data).unwrap();

    let data =
        fs::read_to_string(source.join("api/core/inc/fmod_common.h")).expect("cannot read file");
    let header = fmod_common::parse(&data).unwrap();
    api.opaque_types.extend(header.opaque_types);
    api.type_aliases.extend(header.type_aliases);
    api.constants.extend(header.constants);
    api.enumerations.extend(header.enumerations);
    api.callbacks.extend(header.callbacks);
    api.flags.extend(header.flags);

    let data =
        fs::read_to_string(source.join("api/core/inc/fmod_codec.h")).expect("cannot read file");
    let header = fmod_codec::parse(&data).unwrap();
    api.opaque_types.extend(header.opaque_types);
    api.constants.extend(header.constants);
    api.callbacks.extend(header.callbacks);
    api.flags.extend(header.flags);

    let data =
        fs::read_to_string(source.join("api/core/inc/fmod_output.h")).expect("cannot read file");
    let header = fmod_output::parse(&data).unwrap();
    api.opaque_types.extend(header.opaque_types);
    api.constants.extend(header.constants);
    api.callbacks.extend(header.callbacks);
    api.flags.extend(header.flags);

    let data =
        fs::read_to_string(source.join("api/core/inc/fmod_dsp.h")).expect("cannot read file");
    let header = fmod_dsp::parse(&data).unwrap();
    api.opaque_types.extend(header.opaque_types);
    api.constants.extend(header.constants);
    api.enumerations.extend(header.enumerations);
    api.callbacks.extend(header.callbacks);
    api.flags.extend(header.flags);

    let data = fs::read_to_string(source.join("api/core/inc/fmod_dsp_effects.h"))
        .expect("cannot read file");
    let header = fmod_dsp_effects::parse(&data).unwrap();
    api.constants.extend(header.constants);
    api.enumerations.extend(header.enumerations);

    let data =
        fs::read_to_string(source.join("api/core/inc/fmod_errors.h")).expect("cannot read file");
    let header = fmod_errors::parse(&data).unwrap();
    println!("FMOD Errors");
    println!("Errors: {}", header.mapping.errors.len());

    // post processing
    api.opaque_types.push(OpaqueType {
        name: "FMOD_STUDIO_SYSTEM".into(),
    });

    let code = ffi::generate_api(api).unwrap();
    fs::write("./src/example.rs", code).unwrap();
}

fn main() {
    generate_lib_fmod("./fmod");
}
