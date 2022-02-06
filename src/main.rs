#[macro_use]
extern crate pest_derive;

use crate::parsers::fmod_studio;
use std::fs;
use std::path::Path;

mod api;
mod parsers;

fn generate_lib_fmod(source: &str) {
    let source = Path::new(source);
    let data =
        fs::read_to_string(source.join("api/studio/inc/fmod_studio.h")).expect("cannot read file");
    let header = fmod_studio::parse(&data).unwrap();
    for function in header.functions {
        println!("{:?} {}", function.return_type, function.name);
    }
}

fn main() {
    generate_lib_fmod("./fmod");
}
