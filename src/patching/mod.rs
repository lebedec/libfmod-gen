use crate::models::Api;

pub mod dictionary;
mod functions;
mod post_processing;
mod structures;

impl Api {
    pub fn patch_all(&mut self) {
        self.apply_postprocessing();
        self.patch_functions();
        self.patch_structures();
    }
}
