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
    vram::nv_bridge,
};
use nv_bridge::*;

/// Backend implementation for NV encoding (trait-based).
pub struct NvEncodeBackend {
    codec: *mut c_void,
}
unsafe impl Send for NvEncodeBackend {}

impl NvEncodeBackend {
    fn create(
        device: *mut c_void,
        luid: i64,
        data_format: i32,
        width: i32,
        height: i32,
        bitrate: i32,
        framerate: i32,
        gop: i32,
    ) -> Result<Box<dyn EncodeBackend>, ()> {
        let codec = unsafe {
            nv_new_encoder(device, luid, data_format, width, height, bitrate, framerate, gop)
        };
        if codec.is_null() {
            return Err(());
        }
        Ok(Box::new(NvEncodeBackend { codec }))
    }
}

impl EncodeBackend for NvEncodeBackend {
    fn encode(
        &mut self,
        tex: *mut c_void,
        ms: i64,
        frames: &mut Vec<EncodeFrame>,
    ) -> Result<(), i32> {
        let result = unsafe {
            nv_encode(
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
        match unsafe { nv_set_bitrate(self.codec, kbs) } {
            0 => Ok(()),
            err => Err(err),
        }
    }

    fn set_framerate(&mut self, framerate: i32) -> Result<(), i32> {
        match unsafe { nv_set_framerate(self.codec, framerate) } {
            0 => Ok(()),
            err => Err(err),
        }
    }

    fn destroy(&mut self) {
        if !self.codec.is_null() {
            unsafe {
                nv_destroy_encoder(self.codec);
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
    NvEncodeBackend::create(device, luid, data_format, width, height, bitrate, framerate, gop)
}

/// Backend implementation for NV decoding (trait-based; full when NVDEC is integrated).
pub struct NvDecodeBackend {
    codec: *mut c_void,
}
unsafe impl Send for NvDecodeBackend {}

impl NvDecodeBackend {
    fn create(device: *mut c_void, luid: i64, codec_id: i32) -> Result<Box<dyn DecodeBackend>, ()> {
        let codec = unsafe { nv_new_decoder(device, luid, codec_id) };
        if codec.is_null() {
            return Err(());
        }
        Ok(Box::new(NvDecodeBackend { codec }))
    }
}

impl DecodeBackend for NvDecodeBackend {
    fn decode(&mut self, data: &[u8], frames: &mut Vec<DecodeFrame>) -> Result<(), i32> {
        let result = unsafe {
            nv_decode(
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
                nv_destroy_decoder(self.codec);
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
    NvDecodeBackend::create(device, luid, codec_id)
}

pub fn encode_calls() -> EncodeCalls {
    EncodeCalls {
        new: nv_new_encoder,
        encode: nv_encode,
        destroy: nv_destroy_encoder,
        test: nv_test_encode,
        set_bitrate: nv_set_bitrate,
        set_framerate: nv_set_framerate,
    }
}

pub fn decode_calls() -> DecodeCalls {
    DecodeCalls {
        new: nv_new_decoder,
        decode: nv_decode,
        destroy: nv_destroy_decoder,
        test: nv_test_decode,
    }
}

pub fn possible_support_encoders() -> Vec<InnerEncodeContext> {
    if nv_encode_driver_support() != 0 {
        return vec![];
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
    if nv_decode_driver_support() != 0 || !nv_IsDecodeImplemented() {
        return vec![];
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

// 实现 cxx bridge 中声明的 Rust 函数，实际检测由 C++ nv_IsEncodeDriverAvailable / nv_IsDecodeDriverAvailable 提供
pub(crate) fn nv_encode_driver_support() -> i32 {
    if nv_IsEncodeDriverAvailable() {
        0
    } else {
        -1
    }
}

pub(crate) fn nv_decode_driver_support() -> i32 {
    if nv_IsDecodeDriverAvailable() {
        0
    } else {
        -1
    }
}

// FFI 兼容函数
pub unsafe extern "C" fn nv_new_encoder(
    handle: *mut c_void,
    luid: i64,
    data_format: i32,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
) -> *mut c_void {
    // 转换编解码器格式
    let codec_id = match data_format {
        0 => 0, // H264
        1 => 1, // H265
        _ => return std::ptr::null_mut(),
    };
    
    // 使用 cxx 接口创建编码器
    nv_CreateEncoder(handle as *mut u8, width, height, codec_id, bitrate, framerate, gop) as *mut c_void
}

pub unsafe extern "C" fn nv_encode(
    encoder: *mut c_void,
    texture: *mut c_void,
    callback: extern "C" fn(*const u8, i32, i32, *const c_void, i64),
    obj: *mut c_void,
    ms: i64,
) -> i32 {
    let encoder_ptr = encoder as *mut NvEncoder;
    let frame = nv_EncodeFrame(encoder_ptr, texture as *mut u8, ms);
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
    nv_FreeEncodedFrame(frame);
    0
}

pub unsafe extern "C" fn nv_destroy_encoder(encoder: *mut c_void) -> i32 {
    let encoder_ptr = encoder as *mut NvEncoder;
    nv_DestroyEncoder(encoder_ptr);
    0
}

pub unsafe extern "C" fn nv_set_bitrate(encoder: *mut c_void, bitrate: i32) -> i32 {
    let encoder_ptr = encoder as *mut NvEncoder;
    nv_SetBitrate(encoder_ptr, bitrate);
    0
}

pub unsafe extern "C" fn nv_set_framerate(encoder: *mut c_void, framerate: i32) -> i32 {
    let encoder_ptr = encoder as *mut NvEncoder;
    nv_SetFramerate(encoder_ptr, framerate);
    0
}

pub unsafe extern "C" fn nv_test_encode(
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
    if !nv_IsEncodeDriverAvailable() || luids_count < 1 {
        return 0;
    }
    const NV_LUID: i64 = 0;
    const VENDOR_NV: i32 = 0;
    if exclude_count > 0 && !excluded_luids.is_null() && !exclude_formats.is_null() {
        for i in 0..exclude_count {
            if *excluded_luids.add(i as usize) == NV_LUID
                && *exclude_formats.add(i as usize) == data_format
            {
                return 0;
            }
        }
    }
    *luids = NV_LUID;
    *vendors = VENDOR_NV;
    *desc_count = 1;
    0
}

pub unsafe extern "C" fn nv_new_decoder(
    device: *mut c_void,
    luid: i64,
    codec_id: i32,
) -> *mut c_void {
    nv_CreateDecoder(device as *mut u8, codec_id) as *mut c_void
}

pub unsafe extern "C" fn nv_decode(
    decoder: *mut c_void,
    data: *mut u8,
    len: i32,
    callback: extern "C" fn(*mut c_void, *mut c_void),
    obj: *mut c_void,
) -> i32 {
    let decoder_ptr = decoder as *mut NvDecoder;
    let frame = nv_DecodeFrame(decoder_ptr, data, len);
    if frame.is_null() {
        return -1;
    }
    let decoded_frame = &*frame;
    callback(decoded_frame.texture as *mut c_void, obj);
    nv_FreeDecodedFrame(frame);
    0
}

pub unsafe extern "C" fn nv_destroy_decoder(decoder: *mut c_void) -> i32 {
    let decoder_ptr = decoder as *mut NvDecoder;
    nv_DestroyDecoder(decoder_ptr);
    0
}

pub unsafe extern "C" fn nv_test_decode(
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
    if !nv_IsDecodeDriverAvailable() || luids_count < 1 {
        return 0;
    }
    const NV_LUID: i64 = 0;
    const VENDOR_NV: i32 = 0;
    if exclude_count > 0 && !excluded_luids.is_null() && !exclude_formats.is_null() {
        for i in 0..exclude_count {
            if *excluded_luids.add(i as usize) == NV_LUID
                && *exclude_formats.add(i as usize) == data_format
            {
                return 0;
            }
        }
    }
    *luids = NV_LUID;
    *vendors = VENDOR_NV;
    *desc_count = 1;
    0
}
