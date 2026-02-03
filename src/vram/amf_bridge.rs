#![allow(non_snake_case)]
#![allow(dead_code)] // GetWidth/GetHeight 等为 C++ 接口预留，Rust 侧暂未使用

#[cxx::bridge]
mod amf_bridge {
    unsafe extern "C++" {
        fn amf_IsDriverAvailable() -> bool;
        fn amf_IsDecodeImplemented() -> bool;
    }

    unsafe extern "C++" {
        type AmfEncoder;
        type AmfDecoder;
        
        // AmfEncoder 方法
        unsafe fn amf_CreateEncoder(device: *mut u8, width: i32, height: i32, codec_id: i32, bitrate: i32, framerate: i32, gop: i32) -> *mut AmfEncoder;
        unsafe fn amf_EncodeFrame(encoder: *mut AmfEncoder, texture: *mut u8, timestamp: i64) -> *mut EncodedFrame;
        unsafe fn amf_DestroyEncoder(encoder: *mut AmfEncoder);
        unsafe fn amf_SetBitrate(encoder: *mut AmfEncoder, bitrate: i32);
        unsafe fn amf_SetFramerate(encoder: *mut AmfEncoder, framerate: i32);
        
        // AmfDecoder 方法
        unsafe fn amf_CreateDecoder(device: *mut u8, codec_id: i32) -> *mut AmfDecoder;
        unsafe fn amf_DecodeFrame(decoder: *mut AmfDecoder, data: *mut u8, length: i32) -> *mut DecodedFrame;
        unsafe fn amf_DestroyDecoder(decoder: *mut AmfDecoder);
        unsafe fn amf_GetWidth(decoder: *mut AmfDecoder) -> i32;
        unsafe fn amf_GetHeight(decoder: *mut AmfDecoder) -> i32;

        unsafe fn amf_FreeEncodedFrame(frame: *mut EncodedFrame);
        unsafe fn amf_FreeDecodedFrame(frame: *mut DecodedFrame);
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

pub use amf_bridge::*;
