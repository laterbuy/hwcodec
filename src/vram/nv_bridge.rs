#![allow(non_snake_case)]

#[cxx::bridge]
mod nv_bridge {
    unsafe extern "C++" {
        fn nv_IsEncodeDriverAvailable() -> bool;
        fn nv_IsDecodeDriverAvailable() -> bool;
        fn nv_IsDecodeImplemented() -> bool;
    }

    unsafe extern "C++" {
        type NvEncoder;
        type NvDecoder;

        unsafe fn nv_CreateEncoder(device: *mut u8, width: i32, height: i32, codec_id: i32, bitrate: i32, framerate: i32, gop: i32) -> *mut NvEncoder;
        unsafe fn nv_EncodeFrame(encoder: *mut NvEncoder, texture: *mut u8, timestamp: i64) -> *mut EncodedFrame;
        unsafe fn nv_DestroyEncoder(encoder: *mut NvEncoder);
        unsafe fn nv_SetBitrate(encoder: *mut NvEncoder, bitrate: i32);
        unsafe fn nv_SetFramerate(encoder: *mut NvEncoder, framerate: i32);

        unsafe fn nv_CreateDecoder(device: *mut u8, codec_id: i32) -> *mut NvDecoder;
        unsafe fn nv_DecodeFrame(decoder: *mut NvDecoder, data: *mut u8, length: i32) -> *mut DecodedFrame;
        unsafe fn nv_DestroyDecoder(decoder: *mut NvDecoder);

        unsafe fn nv_FreeEncodedFrame(frame: *mut EncodedFrame);
        unsafe fn nv_FreeDecodedFrame(frame: *mut DecodedFrame);
    }

    struct EncodedFrame {
        data: *mut u8,
        size: i32,
        is_keyframe: bool,
        timestamp: i64,
    }

    struct DecodedFrame {
        texture: *mut u8,
        width: i32,
        height: i32,
    }
}

pub use nv_bridge::*;

