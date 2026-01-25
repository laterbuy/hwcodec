#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]

#[cfg(windows)]
#[path = "mfx_rust.rs"]
mod mfx_rust;

#[cfg(windows)]
pub use mfx_rust::mfx_driver_support;

#[cfg(not(windows))]
include!(concat!(env!("OUT_DIR"), "/mfx_ffi.rs"));

use crate::{
    common::DataFormat::*,
    vram::inner::{DecodeCalls, EncodeCalls, InnerDecodeContext, InnerEncodeContext},
};

pub fn encode_calls() -> EncodeCalls {
    #[cfg(windows)]
    {
        mfx_rust::encode_calls()
    }
    #[cfg(not(windows))]
    {
        EncodeCalls {
            new: mfx_new_encoder,
            encode: mfx_encode,
            destroy: mfx_destroy_encoder,
            test: mfx_test_encode,
            set_bitrate: mfx_set_bitrate,
            set_framerate: mfx_set_framerate,
        }
    }
}

pub fn decode_calls() -> DecodeCalls {
    #[cfg(windows)]
    {
        mfx_rust::decode_calls()
    }
    #[cfg(not(windows))]
    {
        DecodeCalls {
            new: mfx_new_decoder,
            decode: mfx_decode,
            destroy: mfx_destroy_decoder,
            test: mfx_test_decode,
        }
    }
}

pub fn possible_support_encoders() -> Vec<InnerEncodeContext> {
    #[cfg(windows)]
    {
        if mfx_driver_support() != 0 {
            return vec![];
        }
    }
    #[cfg(not(windows))]
    {
        if unsafe { mfx_driver_support() } != 0 {
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
        if mfx_driver_support() != 0 {
            return vec![];
        }
    }
    #[cfg(not(windows))]
    {
        if unsafe { mfx_driver_support() } != 0 {
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
