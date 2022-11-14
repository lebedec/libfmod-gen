use crate::Api;

impl Api {
    pub fn override_functions(&mut self) {
        self.overriding.insert(
            "FMOD_Studio_System_LoadBankMemory".to_string(),
            quote! {
                pub fn load_bank_memory(
                    &self,
                    buffer: &[u8],
                    flags: ffi::FMOD_STUDIO_LOAD_BANK_FLAGS,
                ) -> Result<Bank, Error> {
                    unsafe {
                        let mut bank = null_mut();
                        match ffi::FMOD_Studio_System_LoadBankMemory(
                            self.pointer,
                            buffer.as_ptr() as *const std::os::raw::c_char,
                            buffer.len() as std::os::raw::c_int,
                            LoadMemoryMode::Memory.into(),
                            flags,
                            &mut bank,
                        ) {
                            ffi::FMOD_OK => Ok(Bank::from(bank)),
                            error => Err(err_fmod!("FMOD_Studio_System_LoadBankMemory", error)),
                        }
                    }
                }
            },
        );
        self.overriding.insert(
            "FMOD_Studio_Bank_GetPath".to_string(),
            quote! {
                pub fn get_path(&self) -> Result<String, Error> {
                    unsafe {
                        let mut retrieved = i32::default();
                        match ffi::FMOD_Studio_Bank_GetPath(self.pointer, null_mut(), 0, &mut retrieved) {
                            ffi::FMOD_OK => {
                                let mut buf = vec![0u8; retrieved as usize];
                                match ffi::FMOD_Studio_Bank_GetPath(
                                    self.pointer,
                                    buf.as_mut_ptr() as *mut _,
                                    retrieved,
                                    &mut retrieved
                                ) {
                                    ffi::FMOD_OK => Ok(
                                        CString::from_vec_with_nul_unchecked(buf)
                                            .into_string()
                                            .map_err(Error::String)?
                                    ),
                                    error => Err(err_fmod!("FMOD_Studio_Bank_GetPath", error)),
                                }
                            }
                            error => {
                                Err(err_fmod!("FMOD_Studio_Bank_GetPath", error))
                            }
                        }
                    }
                }
            }
        );
        self.overriding.insert("FMOD_Studio_VCA_GetPath".to_string(), quote! {
            pub fn get_path(&self) -> Result<String, Error> {
                unsafe {
                    let mut retrieved = i32::default();
                    match ffi::FMOD_Studio_VCA_GetPath(self.pointer, null_mut(), 0, &mut retrieved) {
                        ffi::FMOD_OK => {
                            let mut buf = vec![0u8; retrieved as usize];
                            match ffi::FMOD_Studio_VCA_GetPath(
                                self.pointer,
                                buf.as_mut_ptr() as *mut _,
                                retrieved,
                                &mut retrieved,
                            ) {
                                ffi::FMOD_OK => Ok(CString::from_vec_with_nul_unchecked(buf)
                                    .into_string()
                                    .map_err(Error::String)?),
                                error => Err(err_fmod!("FMOD_Studio_VCA_GetPath", error)),
                            }
                        }
                        error => Err(err_fmod!("FMOD_Studio_VCA_GetPath", error)),
                    }
                }
            }
        });
        self.overriding.insert("FMOD_Studio_Bus_GetPath".to_string(), quote! {
            pub fn get_path(&self) -> Result<String, Error> {
                unsafe {
                    let mut retrieved = i32::default();
                    match ffi::FMOD_Studio_Bus_GetPath(self.pointer, null_mut(), 0, &mut retrieved) {
                        ffi::FMOD_OK => {
                            let mut buf = vec![0u8; retrieved as usize];
                            match ffi::FMOD_Studio_Bus_GetPath(
                                self.pointer,
                                buf.as_mut_ptr() as *mut _,
                                retrieved,
                                &mut retrieved,
                            ) {
                                ffi::FMOD_OK => Ok(CString::from_vec_with_nul_unchecked(buf)
                                    .into_string()
                                    .map_err(Error::String)?),
                                error => Err(err_fmod!("FMOD_Studio_Bus_GetPath", error)),
                            }
                        }
                        error => Err(err_fmod!("FMOD_Studio_Bus_GetPath", error)),
                    }
                }
            }
        });
        self.overriding.insert("FMOD_Studio_System_LookupPath".to_string(), quote! {
            pub fn lookup_path(&self, id: Guid) -> Result<String, Error> {
                unsafe {
                    let mut retrieved = i32::default();
                    let id = id.into();
                    match ffi::FMOD_Studio_System_LookupPath(self.pointer, &id, null_mut(), 0, &mut retrieved) {
                        ffi::FMOD_OK => {
                            let mut buf = vec![0u8; retrieved as usize];
                            match ffi::FMOD_Studio_System_LookupPath(
                                self.pointer,
                                &id,
                                buf.as_mut_ptr() as *mut _,
                                retrieved,
                                &mut retrieved,
                            ) {
                                ffi::FMOD_OK => Ok(CString::from_vec_with_nul_unchecked(buf)
                                    .into_string()
                                    .map_err(Error::String)?),
                                error => Err(err_fmod!("FMOD_Studio_System_LookupPath", error)),
                            }
                        }
                        error => Err(err_fmod!("FMOD_Studio_System_LookupPath", error)),
                    }
                }
            }
        });
        self.overriding.insert("FMOD_Studio_EventDescription_GetPath".to_string(), quote! {
            pub fn get_path(&self) -> Result<String, Error> {
                unsafe {
                    let mut retrieved = i32::default();
                    match ffi::FMOD_Studio_EventDescription_GetPath(self.pointer, null_mut(), 0, &mut retrieved) {
                        ffi::FMOD_OK => {
                            let mut buf = vec![0u8; retrieved as usize];
                            match ffi::FMOD_Studio_EventDescription_GetPath(
                                self.pointer,
                                buf.as_mut_ptr() as *mut _,
                                retrieved,
                                &mut retrieved,
                            ) {
                                ffi::FMOD_OK => Ok(CString::from_vec_with_nul_unchecked(buf)
                                    .into_string()
                                    .map_err(Error::String)?),
                                error => Err(err_fmod!("FMOD_Studio_EventDescription_GetPath", error)),
                            }
                        }
                        error => Err(err_fmod!("FMOD_Studio_EventDescription_GetPath", error)),
                    }
                }
            }
        });
        self.overriding.insert(
            "FMOD_Studio_System_IsValid".to_string(),
            quote! {
                pub fn is_valid(&self) -> bool {
                    unsafe {
                        to_bool!(ffi::FMOD_Studio_System_IsValid(self.pointer))
                    }
                }
            },
        );
        self.overriding.insert(
            "FMOD_Studio_EventDescription_IsValid".to_string(),
            quote! {
                pub fn is_valid(&self) -> bool {
                    unsafe {
                        to_bool!(ffi::FMOD_Studio_EventDescription_IsValid(self.pointer))
                    }
                }
            },
        );
        self.overriding.insert(
            "FMOD_Studio_EventInstance_IsValid".to_string(),
            quote! {
                pub fn is_valid(&self) -> bool {
                    unsafe {
                        to_bool!(ffi::FMOD_Studio_EventInstance_IsValid(self.pointer))
                    }
                }
            },
        );
        self.overriding.insert(
            "FMOD_Studio_Bus_IsValid".to_string(),
            quote! {
                pub fn is_valid(&self) -> bool {
                    unsafe {
                        to_bool!(ffi::FMOD_Studio_Bus_IsValid(self.pointer))
                    }
                }
            },
        );
        self.overriding.insert(
            "FMOD_Studio_VCA_IsValid".to_string(),
            quote! {
                pub fn is_valid(&self) -> bool {
                    unsafe {
                        to_bool!(ffi::FMOD_Studio_VCA_IsValid(self.pointer))
                    }
                }
            },
        );
        self.overriding.insert(
            "FMOD_Studio_Bank_IsValid".to_string(),
            quote! {
                pub fn is_valid(&self) -> bool {
                    unsafe {
                        to_bool!(ffi::FMOD_Studio_Bank_IsValid(self.pointer))
                    }
                }
            },
        );
        self.overriding.insert(
            "FMOD_Studio_CommandReplay_IsValid".to_string(),
            quote! {
                pub fn is_valid(&self) -> bool {
                    unsafe {
                        to_bool!(ffi::FMOD_Studio_CommandReplay_IsValid(self.pointer))
                    }
                }
            },
        );
    }
}
