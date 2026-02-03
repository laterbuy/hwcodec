#![allow(non_snake_case)]
#![allow(dead_code)] // GetWidth/GetHeight 等为 C++ 接口预留，Rust 侧暂未使用

#[cxx::bridge]
mod mfx_bridge {
    unsafe extern "C++" {
        fn mfx_IsDriverAvailable() -> bool;
    }

    unsafe extern "C++" {
        type MfxEncoder;
        type MfxDecoder;
        
        // MfxEncoder 方法
        unsafe fn mfx_CreateEncoder(device: *mut u8, width: i32, height: i32, codec_id: i32, bitrate: i32, framerate: i32, gop: i32) -> *mut MfxEncoder;
        unsafe fn mfx_EncodeFrame(encoder: *mut MfxEncoder, texture: *mut u8, timestamp: i64) -> *mut EncodedFrame;
        unsafe fn mfx_DestroyEncoder(encoder: *mut MfxEncoder);
        unsafe fn mfx_SetBitrate(encoder: *mut MfxEncoder, bitrate: i32);
        unsafe fn mfx_SetFramerate(encoder: *mut MfxEncoder, framerate: i32);
        
        // MfxDecoder 方法
        unsafe fn mfx_CreateDecoder(device: *mut u8, codec_id: i32) -> *mut MfxDecoder;
        unsafe fn mfx_DecodeFrame(decoder: *mut MfxDecoder, data: *mut u8, length: i32) -> *mut DecodedFrame;
        unsafe fn mfx_DestroyDecoder(decoder: *mut MfxDecoder);
        unsafe fn mfx_GetWidth(decoder: *mut MfxDecoder) -> i32;
        unsafe fn mfx_GetHeight(decoder: *mut MfxDecoder) -> i32;

        unsafe fn mfx_FreeEncodedFrame(frame: *mut EncodedFrame);
        unsafe fn mfx_FreeDecodedFrame(frame: *mut DecodedFrame);
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

pub use mfx_bridge::*;
