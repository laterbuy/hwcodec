#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]

use std::ffi::c_void;

use crate::{
    common::DataFormat::*,
    vram::inner::{
        DecodeBackend, DecodeCalls, DecodeFrame, EncodeBackend, EncodeCalls, EncodeFrame,
        InnerDecodeContext, InnerEncodeContext,
    },
    vram::mfx_bridge,
};
use mfx_bridge::*;

/// Backend implementation for MFX encoding (trait-based, Rust-owned).
pub struct MfxEncodeBackend {
    codec: *mut c_void,
}
unsafe impl Send for MfxEncodeBackend {}

impl MfxEncodeBackend {
    pub fn create(
        device: *mut c_void,
        _luid: i64,
        data_format: i32,
        width: i32,
        height: i32,
        bitrate: i32,
        framerate: i32,
        gop: i32,
    ) -> Result<Box<dyn EncodeBackend>, ()> {
        let codec = unsafe {
            mfx_new_encoder(device, _luid, data_format, width, height, bitrate, framerate, gop)
        };
        if codec.is_null() {
            return Err(());
        }
        Ok(Box::new(MfxEncodeBackend { codec }))
    }
}

impl EncodeBackend for MfxEncodeBackend {
    fn encode(
        &mut self,
        tex: *mut c_void,
        ms: i64,
        frames: &mut Vec<EncodeFrame>,
    ) -> Result<(), i32> {
        let result = unsafe {
            mfx_encode(
                self.codec,
                tex,
                crate::vram::inner::hwcodec_encode_frame_callback,
                frames as *mut Vec<EncodeFrame> as *mut c_void,
                ms,
            )
        };
        if result != 0 {
            Err(result)
        } else {
            Ok(())
        }
    }

    fn set_bitrate(&mut self, kbs: i32) -> Result<(), i32> {
        match unsafe { mfx_set_bitrate(self.codec, kbs) } {
            0 => Ok(()),
            err => Err(err),
        }
    }

    fn set_framerate(&mut self, framerate: i32) -> Result<(), i32> {
        match unsafe { mfx_set_framerate(self.codec, framerate) } {
            0 => Ok(()),
            err => Err(err),
        }
    }

    fn destroy(&mut self) {
        if !self.codec.is_null() {
            unsafe {
                mfx_destroy_encoder(self.codec);
            }
            self.codec = std::ptr::null_mut();
        }
    }
}

pub fn create_encode_backend(
    device: *mut c_void,
    luid: i64,
    data_format: i32,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
) -> Result<Box<dyn EncodeBackend>, ()> {
    MfxEncodeBackend::create(device, luid, data_format, width, height, bitrate, framerate, gop)
}

/// Backend implementation for MFX decoding (trait-based).
pub struct MfxDecodeBackend {
    codec: *mut c_void,
}
unsafe impl Send for MfxDecodeBackend {}

impl MfxDecodeBackend {
    pub fn create(device: *mut c_void, _luid: i64, codec_id: i32) -> Result<Box<dyn DecodeBackend>, ()> {
        let codec = unsafe { mfx_new_decoder(device, _luid, codec_id) };
        if codec.is_null() {
            return Err(());
        }
        Ok(Box::new(MfxDecodeBackend { codec }))
    }
}

impl DecodeBackend for MfxDecodeBackend {
    fn decode(&mut self, data: &[u8], frames: &mut Vec<DecodeFrame>) -> Result<(), i32> {
        let result = unsafe {
            mfx_decode(
                self.codec,
                data.as_ptr() as *mut u8,
                data.len() as i32,
                crate::vram::inner::hwcodec_decode_frame_callback,
                frames as *mut Vec<DecodeFrame> as *mut c_void,
            )
        };
        if result != 0 {
            Err(result)
        } else {
            Ok(())
        }
    }

    fn destroy(&mut self) {
        if !self.codec.is_null() {
            unsafe {
                mfx_destroy_decoder(self.codec);
            }
            self.codec = std::ptr::null_mut();
        }
    }
}

pub fn create_decode_backend(
    device: *mut c_void,
    luid: i64,
    codec_id: i32,
) -> Result<Box<dyn DecodeBackend>, ()> {
    MfxDecodeBackend::create(device, luid, codec_id)
}

pub fn encode_calls() -> EncodeCalls {
    EncodeCalls {
        new: mfx_new_encoder,
        encode: mfx_encode,
        destroy: mfx_destroy_encoder,
        test: mfx_test_encode,
        set_bitrate: mfx_set_bitrate,
        set_framerate: mfx_set_framerate,
    }
}

pub fn decode_calls() -> DecodeCalls {
    DecodeCalls {
        new: mfx_new_decoder,
        decode: mfx_decode,
        destroy: mfx_destroy_decoder,
        test: mfx_test_decode,
    }
}

pub fn possible_support_encoders() -> Vec<InnerEncodeContext> {
    if mfx_driver_support() != 0 {
        return vec![];
    }
    let data_formats = vec![H264, H265];
    let mut v = vec![];
    for data_format in data_formats.iter() {
        v.push(InnerEncodeContext {
            format: data_format.clone(),
        });
    }
    v
}

pub fn possible_support_decoders() -> Vec<InnerDecodeContext> {
    if mfx_driver_support() != 0 {
        return vec![];
    }
    let data_formats = vec![H264, H265];
    let mut v = vec![];
    for data_format in data_formats.iter() {
        v.push(InnerDecodeContext {
            data_format: data_format.clone(),
        });
    }
    v
}

pub(crate) fn mfx_driver_support() -> i32 {
    if mfx_IsDriverAvailable() {
        0
    } else {
        -1
    }
}

pub unsafe extern "C" fn mfx_new_encoder(
    handle: *mut c_void,
    _luid: i64,
    data_format: i32,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
) -> *mut c_void {
    let codec_id = match data_format {
        0 => 0,
        1 => 1,
        _ => return std::ptr::null_mut(),
    };
    mfx_CreateEncoder(handle as *mut u8, width, height, codec_id, bitrate, framerate, gop)
        as *mut c_void
}

pub unsafe extern "C" fn mfx_encode(
    encoder: *mut c_void,
    texture: *mut c_void,
    callback: extern "C" fn(*const u8, i32, i32, *const c_void, i64),
    obj: *mut c_void,
    ms: i64,
) -> i32 {
    let encoder_ptr = encoder as *mut MfxEncoder;
    let frame = mfx_EncodeFrame(encoder_ptr, texture as *mut u8, ms);
    if frame.is_null() {
        return -1;
    }
    let encoded_frame = &*frame;
    callback(
        encoded_frame.data,
        encoded_frame.size,
        encoded_frame.is_keyframe as i32,
        obj,
        encoded_frame.timestamp,
    );
    mfx_FreeEncodedFrame(frame);
    0
}

pub unsafe extern "C" fn mfx_destroy_encoder(encoder: *mut c_void) -> i32 {
    mfx_DestroyEncoder(encoder as *mut MfxEncoder);
    0
}

pub unsafe extern "C" fn mfx_set_bitrate(encoder: *mut c_void, bitrate: i32) -> i32 {
    mfx_SetBitrate(encoder as *mut MfxEncoder, bitrate);
    0
}

pub unsafe extern "C" fn mfx_set_framerate(encoder: *mut c_void, framerate: i32) -> i32 {
    mfx_SetFramerate(encoder as *mut MfxEncoder, framerate);
    0
}

pub unsafe extern "C" fn mfx_test_encode(
    luids: *mut i64,
    vendors: *mut i32,
    luids_count: i32,
    desc_count: *mut i32,
    data_format: i32,
    _width: i32,
    _height: i32,
    _bitrate: i32,
    _framerate: i32,
    _gop: i32,
    excluded_luids: *const i64,
    exclude_formats: *const i32,
    exclude_count: i32,
) -> i32 {
    if luids.is_null() || vendors.is_null() || desc_count.is_null() {
        return -1;
    }
    *desc_count = 0;
    if !mfx_IsDriverAvailable() || luids_count < 1 {
        return 0;
    }
    const MFX_LUID: i64 = 0;
    const VENDOR_MFX: i32 = 2;
    if exclude_count > 0 && !excluded_luids.is_null() && !exclude_formats.is_null() {
        for i in 0..exclude_count {
            if *excluded_luids.add(i as usize) == MFX_LUID
                && *exclude_formats.add(i as usize) == data_format
            {
                return 0;
            }
        }
    }
    *luids = MFX_LUID;
    *vendors = VENDOR_MFX;
    *desc_count = 1;
    0
}

pub unsafe extern "C" fn mfx_new_decoder(
    device: *mut c_void,
    _luid: i64,
    codec_id: i32,
) -> *mut c_void {
    mfx_CreateDecoder(device as *mut u8, codec_id) as *mut c_void
}

pub unsafe extern "C" fn mfx_decode(
    decoder: *mut c_void,
    data: *mut u8,
    len: i32,
    callback: extern "C" fn(*mut c_void, *mut c_void),
    obj: *mut c_void,
) -> i32 {
    let decoder_ptr = decoder as *mut MfxDecoder;
    let frame = mfx_DecodeFrame(decoder_ptr, data, len);
    if frame.is_null() {
        return -1;
    }
    let decoded_frame = &*frame;
    callback(decoded_frame.texture as *mut c_void, obj);
    mfx_FreeDecodedFrame(frame);
    0
}

pub unsafe extern "C" fn mfx_destroy_decoder(decoder: *mut c_void) -> i32 {
    mfx_DestroyDecoder(decoder as *mut MfxDecoder);
    0
}

pub unsafe extern "C" fn mfx_test_decode(
    luids: *mut i64,
    vendors: *mut i32,
    luids_count: i32,
    desc_count: *mut i32,
    data_format: i32,
    _data: *mut u8,
    _len: i32,
    excluded_luids: *const i64,
    exclude_formats: *const i32,
    exclude_count: i32,
) -> i32 {
    if luids.is_null() || vendors.is_null() || desc_count.is_null() {
        return -1;
    }
    *desc_count = 0;
    if !mfx_IsDriverAvailable() || luids_count < 1 {
        return 0;
    }
    const MFX_LUID: i64 = 0;
    const VENDOR_MFX: i32 = 2;
    if exclude_count > 0 && !excluded_luids.is_null() && !exclude_formats.is_null() {
        for i in 0..exclude_count {
            if *excluded_luids.add(i as usize) == MFX_LUID
                && *exclude_formats.add(i as usize) == data_format
            {
                return 0;
            }
        }
    }
    *luids = MFX_LUID;
    *vendors = VENDOR_MFX;
    *desc_count = 1;
    0
}
