#![allow(non_snake_case)]

use crate::common::{DataFormat, DecodeCallback, EncodeCallback};
use std::os::raw::{c_int, c_void};

// Frame types used by backends and by encode/decode API (moved here to avoid circular deps)
#[derive(Clone, Default)]
pub struct EncodeFrame {
    pub data: Vec<u8>,
    pub pts: i64,
    pub key: i32,
}

#[derive(Default)]
pub struct DecodeFrame {
    pub texture: *mut c_void,
    pub width: i32,
    pub height: i32,
}

/// Backend trait for encoding: Rust-owned API instead of C function table.
pub trait EncodeBackend: Send {
    fn encode(
        &mut self,
        tex: *mut c_void,
        ms: i64,
        frames: &mut Vec<EncodeFrame>,
    ) -> Result<(), i32>;
    fn set_bitrate(&mut self, kbs: i32) -> Result<(), i32>;
    fn set_framerate(&mut self, framerate: i32) -> Result<(), i32>;
    fn destroy(&mut self);
}

/// Backend trait for decoding: Rust-owned API instead of C function table.
pub trait DecodeBackend: Send {
    fn decode(&mut self, data: &[u8], frames: &mut Vec<DecodeFrame>) -> Result<(), i32>;
    fn destroy(&mut self);
}

/// C-compatible callback used by backends when calling into C++ encode (obj = *mut Vec<EncodeFrame>).
#[no_mangle]
pub extern "C" fn hwcodec_encode_frame_callback(
    data: *const u8,
    size: c_int,
    key: i32,
    obj: *const c_void,
    pts: i64,
) {
    if obj.is_null() || data.is_null() {
        return;
    }
    unsafe {
        let frames = &mut *(obj as *const c_void as *mut Vec<EncodeFrame>);
        frames.push(EncodeFrame {
            data: std::slice::from_raw_parts(data, size.max(0) as usize).to_vec(),
            pts,
            key,
        });
    }
}

// C-compatible callback used by backends when calling into C++ decode (obj = *mut Vec<DecodeFrame>).
extern "C" {
    fn hwcodec_get_d3d11_texture_width_height(
        texture: *mut c_void,
        width: *mut i32,
        height: *mut i32,
    );
}

#[no_mangle]
pub extern "C" fn hwcodec_decode_frame_callback(texture: *mut c_void, obj: *mut c_void) {
    if obj.is_null() {
        return;
    }
    let frames = unsafe { &mut *(obj as *mut Vec<DecodeFrame>) };
    let mut width = 0;
    let mut height = 0;
    unsafe {
        hwcodec_get_d3d11_texture_width_height(texture, &mut width, &mut height);
    }
    frames.push(DecodeFrame {
        texture,
        width,
        height,
    });
}

// --- Legacy C function types (still used by available() / test path) ---

pub type NewEncoderCall = unsafe extern "C" fn(
    hdl: *mut c_void,
    luid: i64,
    codecID: i32,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
) -> *mut c_void;

pub type EncodeCall = unsafe extern "C" fn(
    encoder: *mut c_void,
    tex: *mut c_void,
    callback: EncodeCallback,
    obj: *mut c_void,
    ms: i64,
) -> c_int;

pub type NewDecoderCall =
    unsafe extern "C" fn(device: *mut c_void, luid: i64, dataFormat: i32) -> *mut c_void;

pub type DecodeCall = unsafe extern "C" fn(
    decoder: *mut c_void,
    data: *mut u8,
    length: i32,
    callback: DecodeCallback,
    obj: *mut c_void,
) -> c_int;

pub type TestEncodeCall = unsafe extern "C" fn(
    outLuids: *mut i64,
    outVendors: *mut i32,
    maxDescNum: i32,
    outDescNum: *mut i32,
    dataFormat: i32,
    width: i32,
    height: i32,
    kbs: i32,
    framerate: i32,
    gop: i32,
    excludedLuids: *const i64,
    excludeFormats: *const i32,
    excludeCount: i32,
) -> c_int;

pub type TestDecodeCall = unsafe extern "C" fn(
    outLuids: *mut i64,
    outVendors: *mut i32,
    maxDescNum: i32,
    outDescNum: *mut i32,
    dataFormat: i32,
    data: *mut u8,
    length: i32,
    excludedLuids: *const i64,
    excludeFormats: *const i32,
    excludeCount: i32,
) -> c_int;

pub type IVCall = unsafe extern "C" fn(v: *mut c_void) -> c_int;

pub type IVICall = unsafe extern "C" fn(v: *mut c_void, i: i32) -> c_int;

#[allow(dead_code)] // new, encode, destroy, set_* only used via trait backends; test used by available()
pub struct EncodeCalls {
    pub new: NewEncoderCall,
    pub encode: EncodeCall,
    pub destroy: IVCall,
    pub test: TestEncodeCall,
    pub set_bitrate: IVICall,
    pub set_framerate: IVICall,
}
#[allow(dead_code)] // new, decode, destroy only used via trait backends; test used by available()
pub struct DecodeCalls {
    pub new: NewDecoderCall,
    pub decode: DecodeCall,
    pub destroy: IVCall,
    pub test: TestDecodeCall,
}

pub struct InnerEncodeContext {
    pub format: DataFormat,
}

pub struct InnerDecodeContext {
    pub data_format: DataFormat,
}
