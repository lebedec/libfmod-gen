#[macro_use]
extern crate pest_derive;

use crate::parsers::{fmod, fmod_common, fmod_studio, fmod_studio_common};
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
}

fn main() {
    generate_lib_fmod("./fmod");
}
