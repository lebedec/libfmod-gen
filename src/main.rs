#[macro_use]
extern crate pest_derive;

use crate::parsers::{
    fmod, fmod_common, fmod_dsp, fmod_dsp_effects, fmod_output, fmod_studio, fmod_studio_common,
};
use std::fs;
use std::path::Path;

mod models;
mod parsers;
mod repr;

fn generate_lib_fmod(source: &str) {
    let source = Path::new(source);
    let data =
        fs::read_to_string(source.join("api/studio/inc/fmod_studio.h")).expect("cannot read file");
    let header = fmod_studio::parse(&data).unwrap();
    for function in header.functions {
        println!("{:?} {}", function.return_type, function.name);
    }

    let data = fs::read_to_string(source.join("api/studio/inc/fmod_studio_common.h"))
        .expect("cannot read file");
    let header = fmod_studio_common::parse(&data).unwrap();
    println!("FMOD Studio Common");
    println!("Opaque Types: {}", header.opaque_types.len());
    println!("Structures: {}", header.structures.len());
    println!("Constants: {}", header.constants.len());
    println!("Flags: {}", header.flags.len());
    println!("Enumerations: {}", header.enumerations.len());
    println!("Callbacks: {}", header.callbacks.len());

    let data = fs::read_to_string(source.join("api/core/inc/fmod.h")).expect("cannot read file");
    let header = fmod::parse(&data).unwrap();
    for function in header.functions {
        // println!("{:?} {}", function.return_type, function.name);
    }

    let data =
        fs::read_to_string(source.join("api/core/inc/fmod_common.h")).expect("cannot read file");
    let header = fmod_common::parse(&data).unwrap();
    println!("FMOD Common");
    println!("Opaque Types: {}", header.opaque_types.len());
    println!("Structures: {}", header.structures.len());
    println!("Constants: {}", header.constants.len());
    println!("Flags: {}", header.flags.len());
    println!("Enumerations: {}", header.enumerations.len());
    println!("Callbacks: {}", header.callbacks.len());
    println!("Type Aliases: {}", header.type_aliases.len());
    for alias in header.type_aliases {
        println!("{} {:?}", alias.name, alias.base_type);
    }

    let data =
        fs::read_to_string(source.join("api/core/inc/fmod_output.h")).expect("cannot read file");
    let header = fmod_output::parse(&data).unwrap();
    println!("FMOD Output");
    println!("Opaque Types: {}", header.opaque_types.len());
    println!("Structures: {}", header.structures.len());
    println!("Constants: {}", header.constants.len());
    println!("Flags: {}", header.flags.len());
    println!("Callbacks: {}", header.callbacks.len());

    let data =
        fs::read_to_string(source.join("api/core/inc/fmod_dsp.h")).expect("cannot read file");
    let header = fmod_dsp::parse(&data).unwrap();
    println!("FMOD DSP");
    println!("Opaque Types: {}", header.opaque_types.len());
    println!("Structures: {}", header.structures.len());
    println!("Constants: {}", header.constants.len());
    println!("Flags: {}", header.flags.len());
    println!("Callbacks: {}", header.callbacks.len());
    println!("Enumerations: {}", header.enumerations.len());

    let data = fs::read_to_string(source.join("api/core/inc/fmod_dsp_effects.h"))
        .expect("cannot read file");
    let header = fmod_dsp_effects::parse(&data).unwrap();
    println!("FMOD DSP Effects");
    println!("Structures: {}", header.structures.len());
    println!("Constants: {}", header.constants.len());
    println!("Enumerations: {}", header.enumerations.len());
}

fn main() {
    generate_lib_fmod("./fmod");
}
