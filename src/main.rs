#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate quote;

extern crate proc_macro;

#[macro_use]
extern crate pest_derive;

use crate::generators::{ffi, lib};
use crate::models::{Api, Error, Modifier, OpaqueType};
use crate::parsers::{
    fmod, fmod_codec, fmod_common, fmod_docs, fmod_dsp, fmod_dsp_effects, fmod_errors, fmod_output,
    fmod_studio, fmod_studio_common,
};
use std::path::Path;
use std::{env, fs};

mod generators;
mod models;
mod overriding;
mod parsers;
mod repr;

fn generate_lib_fmod(source: &str, destination: &str) -> Result<(), Error> {
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

    api.modifiers = fmod_docs::parse_parameter_modifiers(&[
        source.join("doc/FMOD API User Manual/core-api-system.html"),
        source.join("doc/FMOD API User Manual/core-api-soundgroup.html"),
        source.join("doc/FMOD API User Manual/core-api-sound.html"),
        source.join("doc/FMOD API User Manual/core-api-reverb3d.html"),
        source.join("doc/FMOD API User Manual/core-api-geometry.html"),
        source.join("doc/FMOD API User Manual/core-api-dspconnection.html"),
        source.join("doc/FMOD API User Manual/core-api-dsp.html"),
        source.join("doc/FMOD API User Manual/core-api-channelgroup.html"),
        source.join("doc/FMOD API User Manual/core-api-channelcontrol.html"),
        source.join("doc/FMOD API User Manual/core-api-channel.html"),
        source.join("doc/FMOD API User Manual/core-api-common.html"),
        source.join("doc/FMOD API User Manual/plugin-api-codec.html"),
        source.join("doc/FMOD API User Manual/plugin-api-dsp.html"),
        source.join("doc/FMOD API User Manual/plugin-api-output.html"),
        source.join("doc/FMOD API User Manual/studio-api-bank.html"),
        source.join("doc/FMOD API User Manual/studio-api-bus.html"),
        source.join("doc/FMOD API User Manual/studio-api-commandreplay.html"),
        source.join("doc/FMOD API User Manual/studio-api-common.html"),
        source.join("doc/FMOD API User Manual/studio-api-eventdescription.html"),
        source.join("doc/FMOD API User Manual/studio-api-eventinstance.html"),
        source.join("doc/FMOD API User Manual/studio-api-system.html"),
        source.join("doc/FMOD API User Manual/studio-api-vca.html"),
    ])?;

    // POST PROCESSING

    if !api
        .opaque_types
        .iter()
        .any(|opaque_type| opaque_type.name == "FMOD_STUDIO_SYSTEM")
    {
        api.opaque_types.push(OpaqueType {
            name: "FMOD_STUDIO_SYSTEM".into(),
        });
    }
    let not_specified_output = &[
        "FMOD_Studio_CommandReplay_GetSystem+system",
        "FMOD_Studio_CommandReplay_GetCommandString+buffer",
        "FMOD_Studio_CommandReplay_GetPaused+paused",
        "FMOD_Studio_CommandReplay_GetUserData+userdata",
        "FMOD_Studio_EventDescription_Is3D+is3D",
        "FMOD_Studio_System_GetCoreSystem+coresystem",
        "FMOD_System_GetNumNestedPlugins+count",
    ];
    for key in not_specified_output {
        api.modifiers.insert(key.to_string(), Modifier::Out);
    }
    let not_output = &[
        "FMOD_System_Set3DNumListeners+numlisteners",
        "FMOD_Channel_GetMixMatrix+inchannel_hop",
        "FMOD_ChannelGroup_GetMixMatrix+inchannel_hop",
    ];
    for key in not_output {
        api.modifiers.remove(&key.to_string());
    }

    api.conversions.insert("FMOD_DSP_PARAMETER_FFT".to_string(), quote! {
        impl TryFrom<Dsp> for DspParameterFft {
            type Error = Error;
            fn try_from(dsp: Dsp) -> Result<Self, Self::Error> {
                match dsp.get_type() {
                    Ok(DspType::Fft) => {
                        let (ptr, _, _) = dsp.get_parameter_data(ffi::FMOD_DSP_FFT_SPECTRUMDATA, 0)?;
                        let fft = unsafe {
                            *(ptr as *const ffi::FMOD_DSP_PARAMETER_FFT)
                        };
                        DspParameterFft::try_from(fft)
                    },
                    _ => Err(Error::NotDspFft)
                }
            }
        }
    });
    api.override_functions();

    println!("FMOD API");
    println!("Opaque Types: {}", api.opaque_types.len());
    println!("Type Aliases: {}", api.type_aliases.len());
    println!(
        "Structures: {} (Fields: {})",
        api.structures.len(),
        api.structures
            .iter()
            .flat_map(|structure| &structure.fields)
            .count()
    );
    println!("Constants: {}", api.constants.len());
    println!(
        "Flags: {} (Options: {})",
        api.flags.len(),
        api.flags.iter().flat_map(|flags| &flags.flags).count()
    );
    println!(
        "Enumerations: {} (Variants: {})",
        api.enumerations.len(),
        api.enumerations
            .iter()
            .flat_map(|enumeration| &enumeration.enumerators)
            .count()
    );
    println!("Callbacks: {}", api.callbacks.len());
    println!(
        "Functions: {} (Arguments: {})",
        api.functions
            .iter()
            .flat_map(|(_, functions)| functions)
            .count(),
        api.functions
            .iter()
            .flat_map(|(_, functions)| functions)
            .flat_map(|function| &function.arguments)
            .count()
    );
    println!("Parameter Modifiers: {}", api.modifiers.len());
    println!("Errors: {}", api.errors.errors.len());

    let destination = Path::new(destination);
    let code = ffi::generate(&api)?;
    fs::write(destination.join("src/ffi.rs"), code)?;
    let code = lib::generate(&api)?;
    fs::write(destination.join("src/lib.rs"), code)?;

    Ok(())
}

const FMOD_SDK_PATH: &str = "C:\\Program Files (x86)\\FMOD SoundSystem\\FMOD Studio API Windows";

fn main() {
    let mut args = env::args();
    let source = args.nth(1).unwrap_or(FMOD_SDK_PATH.to_string());
    let destination = args.nth(2).unwrap_or("../libfmod".to_string());
    if let Err(error) = generate_lib_fmod(&source, &destination) {
        println!("Unable to generate libfmod, {:?}", error);
    }
}
