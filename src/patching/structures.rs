use crate::Api;

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
}
