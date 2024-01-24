use crate::patching::dictionary::RENAMES;
use crate::Api;
use convert_case::{Case, Casing};
use quote::__private::TokenStream;

impl Api {
    pub fn patch_structures(&mut self) {
        self.structure_patches.insert("FMOD_DSP_PARAMETER_FFT".to_string(), quote! {
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
    }

    pub fn patch_structure_name(key: &str) -> String {
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
        name.to_string()
    }

    pub fn patch_ffi_structure_default(key: &str) -> Option<TokenStream> {
        let definition = match key {
            "FMOD_STUDIO_ADVANCEDSETTINGS" => quote! {
                impl Default for FMOD_STUDIO_ADVANCEDSETTINGS {
                    fn default() -> Self {
                        let mut value: Self = unsafe { std::mem::zeroed() };
                        value.cbsize = std::mem::size_of::<FMOD_STUDIO_ADVANCEDSETTINGS>() as _;
                        value
                    }
                }
            },
            "FMOD_ADVANCEDSETTINGS" => quote! {
                impl Default for FMOD_ADVANCEDSETTINGS {
                    fn default() -> Self {
                        let mut value: Self = unsafe { std::mem::zeroed() };
                        value.cbSize = std::mem::size_of::<FMOD_ADVANCEDSETTINGS>() as _;
                        value
                    }
                }
            },
            "FMOD_CREATESOUNDEXINFO" => quote! {
                impl Default for FMOD_CREATESOUNDEXINFO {
                    fn default() -> Self {
                        let mut value: Self = unsafe { std::mem::zeroed() };
                        value.cbsize = std::mem::size_of::<FMOD_CREATESOUNDEXINFO>() as _;
                        value
                    }
                }
            },
            _ => return None,
        };
        Some(definition)
    }
}
