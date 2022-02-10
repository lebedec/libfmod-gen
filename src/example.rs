#![allow(non_camel_case_types)]
use std::os::raw;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_STUDIO_EVENTDESCRIPTION {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_STUDIO_EVENTINSTANCE {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_STUDIO_BUS {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_STUDIO_VCA {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_STUDIO_BANK {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_STUDIO_COMMANDREPLAY {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_SYSTEM {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_SOUND {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_CHANNELCONTROL {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_CHANNEL {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_CHANNELGROUP {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_SOUNDGROUP {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_REVERB3D {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_DSP {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_DSPCONNECTION {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_POLYGON {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_GEOMETRY {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_SYNCPOINT {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_ASYNCREADINFO {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_OUTPUT_STATE {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_OUTPUT_OBJECT3DINFO {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_DSP_STATE {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_DSP_BUFFER_ARRAY {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FMOD_COMPLEX {
    _unused: [u8; 0],
}
pub const FMOD_STUDIO_LOAD_MEMORY_ALIGNMENT: raw::c_uint = 32;
pub const FMOD_MAX_CHANNEL_WIDTH: raw::c_uint = 32;
pub const FMOD_MAX_SYSTEMS: raw::c_uint = 8;
pub const FMOD_MAX_LISTENERS: raw::c_uint = 8;
pub const FMOD_REVERB_MAXINSTANCES: raw::c_uint = 4;
pub const FMOD_PORT_INDEX_NONE: raw::c_ulonglong = 0xFFFFFFFFFFFFFFFF;
pub const FMOD_OUTPUT_PLUGIN_VERSION: raw::c_uint = 5;
pub const FMOD_PLUGIN_SDK_VERSION: raw::c_uint = 110;
pub const FMOD_DSP_GETPARAM_VALUESTR_LENGTH: raw::c_uint = 32;
pub const FMOD_DSP_LOUDNESS_METER_HISTOGRAM_SAMPLES: raw::c_uint = 66;
