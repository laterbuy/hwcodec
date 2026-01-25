#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]

#[cfg(windows)]
#[path = "nv_rust.rs"]
mod nv_rust;

#[cfg(windows)]
pub use nv_rust::{nv_encode_driver_support, nv_decode_driver_support};

#[cfg(not(windows))]
include!(concat!(env!("OUT_DIR"), "/nv_ffi.rs"));

use crate::{
    common::DataFormat::*,
    vram::inner::{DecodeCalls, EncodeCalls, InnerDecodeContext, InnerEncodeContext},
};

pub fn encode_calls() -> EncodeCalls {
    #[cfg(windows)]
    {
        nv_rust::encode_calls()
    }
    #[cfg(not(windows))]
    {
        EncodeCalls {
            new: nv_new_encoder,
            encode: nv_encode,
            destroy: nv_destroy_encoder,
            test: nv_test_encode,
            set_bitrate: nv_set_bitrate,
            set_framerate: nv_set_framerate,
        }
    }
}

pub fn decode_calls() -> DecodeCalls {
    #[cfg(windows)]
    {
        nv_rust::decode_calls()
    }
    #[cfg(not(windows))]
    {
        DecodeCalls {
            new: nv_new_decoder,
            decode: nv_decode,
            destroy: nv_destroy_decoder,
            test: nv_test_decode,
        }
    }
}

pub fn possible_support_encoders() -> Vec<InnerEncodeContext> {
    #[cfg(windows)]
    {
        if nv_encode_driver_support() != 0 {
            return vec![];
        }
    }
    #[cfg(not(windows))]
    {
        if unsafe { nv_encode_driver_support() } != 0 {
            return vec![];
        }
    }
    let dataFormats = vec![H264, H265];
    let mut v = vec![];
    for dataFormat in dataFormats.iter() {
        v.push(InnerEncodeContext {
            format: dataFormat.clone(),
        });
    }
    v
}

pub fn possible_support_decoders() -> Vec<InnerDecodeContext> {
    #[cfg(windows)]
    {
        if nv_decode_driver_support() != 0 {
            return vec![];
        }
    }
    #[cfg(not(windows))]
    {
        if unsafe { nv_encode_driver_support() } != 0 {
            return vec![];
        }
    }
    let dataFormats = vec![H264, H265];
    let mut v = vec![];
    for dataFormat in dataFormats.iter() {
        v.push(InnerDecodeContext {
            data_format: dataFormat.clone(),
        });
    }
    v
}
