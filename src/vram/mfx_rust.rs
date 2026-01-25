//! Intel MediaSDK (MFX) 的 Rust 实现
//! Windows平台使用 C 包装层（mfx_encode.cpp/mfx_decode.cpp）调用 SDK，所有业务逻辑在 Rust 中实现

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]

use crate::{
    common::DataFormat::*,
    vram::inner::{DecodeCalls, EncodeCalls, InnerDecodeContext, InnerEncodeContext},
};
use serde_derive::{Deserialize, Serialize};
use std::ffi::{c_int, c_void};

// 从common.h导入的类型（包括EncodeCallback和DecodeCallback）
include!(concat!(env!("OUT_DIR"), "/common_ffi.rs"));

// MFX SDK C 包装层 FFI 绑定
#[cfg(windows)]
mod mfx_wrapper_ffi {
    use super::*;
    
    // 手动声明 mfx_wrapper 函数（因为 bindgen 可能无法正确生成）
    extern "C" {
        // Session 操作
        pub fn mfx_wrapper_session_init(session: *mut *mut c_void) -> i32;
        pub fn mfx_wrapper_session_set_handle_d3d11(session: *mut c_void, device: *mut c_void) -> i32;
        pub fn mfx_wrapper_session_set_frame_allocator(session: *mut c_void, allocator: *mut c_void) -> i32;
        pub fn mfx_wrapper_session_close(session: *mut c_void);
        pub fn mfx_wrapper_get_simple_frame_allocator() -> *mut c_void;
        
        // 编码器操作
        pub fn mfx_wrapper_create_encoder(session: *mut c_void, encoder: *mut *mut c_void) -> i32;
        pub fn mfx_wrapper_encoder_close(encoder: *mut c_void);
        pub fn mfx_wrapper_encoder_get_video_param(encoder: *mut c_void, params: *mut c_void) -> i32;
        pub fn mfx_wrapper_encoder_reset(encoder: *mut c_void, params: *mut c_void) -> i32;
        pub fn mfx_wrapper_encoder_encode_frame_async(
            encoder: *mut c_void,
            surface: *mut c_void,
            bitstream: *mut c_void,
            syncp: *mut c_void,
        ) -> i32;
        
        // 解码器操作
        pub fn mfx_wrapper_create_decoder(session: *mut c_void, decoder: *mut *mut c_void) -> i32;
        pub fn mfx_wrapper_decoder_close(decoder: *mut c_void);
        pub fn mfx_wrapper_decoder_query(decoder: *mut c_void, params: *mut c_void, caps: *mut c_void) -> i32;
        pub fn mfx_wrapper_decoder_init(decoder: *mut c_void, params: *mut c_void) -> i32;
        pub fn mfx_wrapper_decoder_decode_frame_async(
            decoder: *mut c_void,
            bitstream: *mut c_void,
            surface_work: *mut c_void,
            surface_out: *mut *mut c_void,
            syncp: *mut c_void,
        ) -> i32;
        
        // Surface 操作
        pub fn mfx_wrapper_surface_get_mem_id(surface: *mut c_void) -> *mut c_void;
        pub fn mfx_wrapper_surface_get_info(surface: *mut c_void, info: *mut c_void) -> i32;
        
        // Bitstream 操作
        pub fn mfx_wrapper_bitstream_init(bitstream: *mut c_void, data: *mut u8, length: u32);
        pub fn mfx_wrapper_bitstream_get_data(bitstream: *mut c_void) -> *mut u8;
        pub fn mfx_wrapper_bitstream_get_length(bitstream: *mut c_void) -> u32;
        pub fn mfx_wrapper_bitstream_get_frame_type(bitstream: *mut c_void) -> u32;
        
        // 同步操作
        pub fn mfx_wrapper_sync_operation(session: *mut c_void, syncp: *mut c_void, timeout: u32) -> i32;
        
        // 高级接口
        pub fn mfx_wrapper_create_encoder_params(
            codec_id: i32,
            width: i32,
            height: i32,
            framerate: i32,
            bitrate_kbps: i32,
            gop: i32,
        ) -> *mut c_void;
        pub fn mfx_wrapper_destroy_encoder_params(params: *mut c_void);
        pub fn mfx_wrapper_align16(value: i32) -> i32;
        pub fn mfx_wrapper_align16_height(value: i32) -> i32;
        pub fn mfx_wrapper_get_free_surface_index(surfaces: *mut c_void, surface_count: i32) -> i32;
        
        // 高级接口
        pub fn mfx_wrapper_encoder_query_and_init(encoder: *mut c_void, params: *mut c_void) -> i32;
        pub fn mfx_wrapper_encoder_query_iosurf(encoder: *mut c_void, params: *mut c_void) -> i32;
        pub fn mfx_wrapper_create_bitstream(max_length: u32) -> *mut c_void;
        pub fn mfx_wrapper_destroy_bitstream(bitstream: *mut c_void);
        pub fn mfx_wrapper_create_surface_array(count: i32, frame_info: *mut c_void) -> *mut c_void;
        pub fn mfx_wrapper_destroy_surface_array(surfaces: *mut c_void);
        pub fn mfx_wrapper_surface_set_mem_id(surface: *mut c_void, mem_id: *mut c_void);
        pub fn mfx_wrapper_create_syncpoint() -> *mut c_void;
        pub fn mfx_wrapper_destroy_syncpoint(syncp: *mut c_void);
        pub fn mfx_wrapper_get_surface_at(surfaces: *mut c_void, index: i32) -> *mut c_void;
        
        // 解码器高级接口
        pub fn mfx_wrapper_decoder_decode_header(decoder: *mut c_void, bitstream: *mut c_void, params: *mut c_void) -> i32;
        pub fn mfx_wrapper_decoder_query_iosurf(decoder: *mut c_void, params: *mut c_void) -> i32;
        pub fn mfx_wrapper_create_decoder_params(codec_id: i32) -> *mut c_void;
        pub fn mfx_wrapper_destroy_decoder_params(params: *mut c_void);
        pub fn mfx_wrapper_decoder_initialize_from_bitstream(
            decoder: *mut c_void,
            bitstream: *mut c_void,
            params: *mut c_void,
            allocator: *mut c_void,
            surfaces: *mut *mut c_void,
            surface_count: *mut i32,
        ) -> i32;
        
        // 帧分配器操作
        pub fn mfx_wrapper_create_d3d11_frame_allocator(device: *mut c_void, allocator: *mut *mut c_void) -> i32;
        pub fn mfx_wrapper_allocator_alloc(allocator: *mut c_void, request: *mut c_void, response: *mut c_void) -> i32;
        pub fn mfx_wrapper_allocator_free(allocator: *mut c_void, response: *mut c_void) -> i32;
        pub fn mfx_wrapper_allocator_release(allocator: *mut c_void);
    }
}

// NativeDevice FFI 函数声明（统一在文件顶部声明，避免重复）
#[cfg(windows)]
extern "C" {
    fn hwcodec_native_device_destroy(handle: *mut c_void);
}

#[cfg(windows)]
use mfx_wrapper_ffi::*;

// MFX 常量定义
#[cfg(windows)]
mod mfx_constants {
    // Codec IDs
    pub const MFX_CODEC_AVC: i32 = 0x001;  // H.264
    pub const MFX_CODEC_HEVC: i32 = 0x002; // H.265
    
    // FourCC
    pub const MFX_FOURCC_NV12: i32 = 0x3231564E; // 'NV12'
    pub const MFX_FOURCC_BGR4: i32 = 0x34424742; // 'BGR4'
    
    // Chroma Format
    pub const MFX_CHROMAFORMAT_YUV420: i32 = 1;
    pub const MFX_CHROMAFORMAT_YUV444: i32 = 3;
    
    // PicStruct
    pub const MFX_PICSTRUCT_PROGRESSIVE: i32 = 1;
    
    // IOPattern
    pub const MFX_IOPATTERN_IN_VIDEO_MEMORY: i32 = 0x0100;
    pub const MFX_IOPATTERN_OUT_VIDEO_MEMORY: i32 = 0x0200;
    
    // Target Usage
    pub const MFX_TARGETUSAGE_BEST_SPEED: i32 = 7;
    
    // Rate Control
    pub const MFX_RATECONTROL_VBR: i32 = 2;
    
    // Profile & Level
    pub const MFX_PROFILE_AVC_MAIN: i32 = 77;
    pub const MFX_LEVEL_AVC_51: i32 = 51;
    pub const MFX_PROFILE_HEVC_MAIN: i32 = 1;
    pub const MFX_LEVEL_HEVC_51: i32 = 153;
    
    // Frame Type
    pub const MFX_FRAMETYPE_I: u32 = 0x0001;
    pub const MFX_FRAMETYPE_IDR: u32 = 0x0004;
}

#[cfg(windows)]
use mfx_constants::*;

// mfxFrameInfo 结构体定义（用于获取 Surface 信息）
#[cfg(windows)]
#[repr(C)]
struct mfxFrameInfo {
    reserved: [u32; 4],
    reserved4: u16,
    bit_depth_luma: u16,
    bit_depth_chroma: u16,
    shift: u16,
    frame_id: [u16; 4], // mfxFrameId 简化
    fourcc: u32,
    width: u16,
    height: u16,
    crop_x: u16,
    crop_y: u16,
    crop_w: u16,
    crop_h: u16,
    frame_rate_ext_n: u32,
    frame_rate_ext_d: u32,
    reserved3: u16,
    aspect_ratio_w: u16,
    aspect_ratio_h: u16,
    pic_struct: u16,
    chroma_format: u16,
    reserved2: u16,
}

// 编码器结构（在 Rust 中管理）
#[cfg(windows)]
pub struct MfxEncoder {
    session: *mut c_void,      // MFXVideoSession*
    encoder: *mut c_void,      // MFXVideoENCODE*
    encoder_params: *mut c_void, // mfxVideoParam*
    native_device: *mut c_void, // NativeDeviceHandle
    surfaces_array: *mut c_void, // mfxFrameSurface1* 数组指针
    surface_count: i32,        // Surface 数量
    bitstream: *mut c_void,     // mfxBitstream*
    bitstream_data: Vec<u8>,     // Bitstream 数据缓冲区
    data_format: crate::common::DataFormat,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
    nv12_texture: *mut c_void,  // ID3D11Texture2D* (NV12 转换纹理)
}

// 解码器结构（在 Rust 中管理）
#[cfg(windows)]
pub struct MfxDecoder {
    session: *mut c_void,      // MFXVideoSession*
    decoder: *mut c_void,      // MFXVideoDECODE*
    decoder_params: *mut c_void, // mfxVideoParam*
    native_device: *mut c_void, // NativeDeviceHandle
    surfaces_array: *mut c_void, // mfxFrameSurface1* 数组指针
    surface_count: i32,        // Surface 数量
    allocator: *mut c_void,     // D3D11FrameAllocator*
    initialized: bool,
    data_format: crate::common::DataFormat,
}

// 辅助函数：检查MFX驱动支持
#[cfg(windows)]
pub fn mfx_driver_support() -> i32 {
    unsafe {
        let mut session: *mut c_void = std::ptr::null_mut();
        let result = mfx_wrapper_session_init(&mut session);
        if result == 0 && !session.is_null() {
            mfx_wrapper_session_close(session);
            return 0; // 支持
        }
        -1 // 不支持
    }
}

#[cfg(not(windows))]
pub fn mfx_driver_support() -> i32 {
    // 非 Windows 平台暂不支持
    -1
}

// 辅助函数：转换编解码器格式
#[cfg(windows)]
fn convert_codec_to_mfx(data_format: i32) -> Option<i32> {
    match data_format {
        0 => Some(MFX_CODEC_AVC),  // H264
        1 => Some(MFX_CODEC_HEVC), // H265
        _ => None,
    }
}

// 辅助函数：创建编码器
#[cfg(windows)]
pub unsafe extern "C" fn mfx_new_encoder(
    handle: *mut c_void,
    luid: i64,
    data_format: i32,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
) -> *mut c_void {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    // 1. 转换编解码器格式
    let codec_id = match convert_codec_to_mfx(data_format) {
        Some(id) => id,
        None => return std::ptr::null_mut(),
    };

    let data_format_enum = match data_format {
        0 => H264,
        1 => H265,
        _ => return std::ptr::null_mut(),
    };

    // 2. 创建 NativeDevice
    extern "C" {
        fn hwcodec_native_device_new(
            luid: i64,
            device: *mut c_void,
            pool_size: i32,
        ) -> *mut c_void;
        fn hwcodec_native_device_get_device(handle: *mut c_void) -> *mut c_void;
    }

    let native_device = hwcodec_native_device_new(luid, handle, 1);
    if native_device.is_null() {
        return std::ptr::null_mut();
    }

    let amf_device = hwcodec_native_device_get_device(native_device);
    if amf_device.is_null() {
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 3. 初始化 MFX Session
    let mut session: *mut c_void = std::ptr::null_mut();
    if mfx_wrapper_session_init(&mut session) != 0 {
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 4. 设置 D3D11 设备句柄
    if mfx_wrapper_session_set_handle_d3d11(session, amf_device) != 0 {
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 5. 设置帧分配器
    let allocator = mfx_wrapper_get_simple_frame_allocator();
    if mfx_wrapper_session_set_frame_allocator(session, allocator) != 0 {
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 6. 创建编码器参数
    let encoder_params = mfx_wrapper_create_encoder_params(
        codec_id,
        width,
        height,
        framerate,
        bitrate,
        gop,
    );
    if encoder_params.is_null() {
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 7. 创建编码器
    let mut encoder: *mut c_void = std::ptr::null_mut();
    if mfx_wrapper_create_encoder(session, &mut encoder) != 0 {
        mfx_wrapper_destroy_encoder_params(encoder_params);
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 8. 查询编码器 Surface 需求
    let surface_count = mfx_wrapper_encoder_query_iosurf(encoder, encoder_params);
    if surface_count < 0 {
        mfx_wrapper_encoder_close(encoder);
        mfx_wrapper_destroy_encoder_params(encoder_params);
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 9. 创建 Surface 数组
    // 注意：需要从 encoder_params 获取 FrameInfo，但这里暂时传 NULL
    // 后续可以从 encoder_params 中提取
    let surfaces_ptr = mfx_wrapper_create_surface_array(surface_count, std::ptr::null_mut());
    if surfaces_ptr.is_null() {
        mfx_wrapper_encoder_close(encoder);
        mfx_wrapper_destroy_encoder_params(encoder_params);
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 10. 查询并初始化编码器
    if mfx_wrapper_encoder_query_and_init(encoder, encoder_params) != 0 {
        mfx_wrapper_destroy_surface_array(surfaces_ptr);
        mfx_wrapper_encoder_close(encoder);
        mfx_wrapper_destroy_encoder_params(encoder_params);
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 11. 获取编码器参数以确定 Bitstream 大小
    // 注意：mfx_wrapper_encoder_get_video_param 在初始化后调用
    // 但我们已经通过 query_and_init 获取了参数，所以可以从 encoder_params 中获取
    // 暂时使用固定大小
    let bitstream_size = 512 * 1024; // 512KB，后续可以从参数获取
    let mut bitstream_data = vec![0u8; bitstream_size];
    
    // 12. 创建 Bitstream 结构
    let bitstream = mfx_wrapper_create_bitstream(bitstream_size as u32);
    if bitstream.is_null() {
        mfx_wrapper_destroy_surface_array(surfaces_ptr);
        mfx_wrapper_encoder_close(encoder);
        mfx_wrapper_destroy_encoder_params(encoder_params);
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }
    
    // 设置 Bitstream 数据指针
    mfx_wrapper_bitstream_init(bitstream, bitstream_data.as_mut_ptr(), bitstream_size as u32);
    
    // 13. 保存 Surface 数组指针（后续通过索引访问）
    // 注意：surfaces_ptr 指向 mfxFrameSurface1 数组的开始
    // 我们保存数组指针，后续通过 mfx_wrapper_get_surface_at 访问单个 Surface
    
    // 14. 创建编码器结构
    let encoder_struct = Box::new(MfxEncoder {
        session,
        encoder,
        encoder_params,
        native_device,
        surfaces_array: surfaces_ptr,
        surface_count,
        bitstream,
        bitstream_data,
        data_format: data_format_enum,
        width,
        height,
        bitrate,
        framerate,
        gop,
        nv12_texture: std::ptr::null_mut(),
    });

    Box::into_raw(encoder_struct) as *mut c_void
}

#[cfg(not(windows))]
pub unsafe extern "C" fn mfx_new_encoder(
    _handle: *mut c_void,
    _luid: i64,
    _data_format: i32,
    _width: i32,
    _height: i32,
    _bitrate: i32,
    _framerate: i32,
    _gop: i32,
) -> *mut c_void {
    std::ptr::null_mut()
}

// 辅助函数：编码
#[cfg(windows)]
pub unsafe extern "C" fn mfx_encode(
    encoder: *mut c_void,
    texture: *mut c_void,
    callback: EncodeCallback,
    obj: *mut c_void,
    ms: i64,
) -> i32 {
    if encoder.is_null() || texture.is_null() {
        return -1;
    }

    let enc = &mut *(encoder as *mut MfxEncoder);

    // 1. 获取空闲 Surface
    let surface_idx = mfx_wrapper_get_free_surface_index(enc.surfaces_array, enc.surface_count);
    if surface_idx < 0 {
        return -1;
    }

    let surface = mfx_wrapper_get_surface_at(enc.surfaces_array, surface_idx);
    if surface.is_null() {
        return -1;
    }

    // 2. 转换 BGRA 纹理到 NV12（使用 NativeDevice）
    extern "C" {
        fn hwcodec_native_device_get_device(handle: *mut c_void) -> *mut c_void;
        fn hwcodec_native_device_bgra_to_nv12(
            handle: *mut c_void,
            bgra_texture: *mut c_void,
            nv12_texture: *mut c_void,
            width: u32,
            height: u32,
            color_space_in: u32,
            color_space_out: u32,
        ) -> i32;
    }

    // 创建或获取 NV12 纹理
    if enc.nv12_texture.is_null() {
        // TODO: 从 D3D11 纹理描述创建 NV12 纹理
        // 暂时使用原始纹理（如果编码器支持 BGRA 输入）
        // 或者需要创建 NV12 纹理
        // 这里简化处理，假设使用 CONFIG_USE_D3D_CONVERT
        use windows::Win32::Graphics::Direct3D11::*;
        use windows::Win32::Graphics::Dxgi::Common::*;
        use windows::core::Interface;
        
        let device = hwcodec_native_device_get_device(enc.native_device);
        let device_ptr: ID3D11Device = Interface::from_raw(device as *mut std::ffi::c_void);
        
        let src_texture: ID3D11Texture2D = Interface::from_raw(texture as *mut std::ffi::c_void);
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        src_texture.GetDesc(&mut desc);
        
        desc.Format = DXGI_FORMAT_NV12;
        desc.MiscFlags = 0u32;
        
        let mut nv12_texture: Option<ID3D11Texture2D> = None;
        let hr = device_ptr.CreateTexture2D(&desc, None, Some(&mut nv12_texture));
        if hr.is_err() || nv12_texture.is_none() {
            std::mem::forget(src_texture);
            std::mem::forget(device_ptr);
            return -1;
        }
        
        let nv12 = nv12_texture.unwrap();
        enc.nv12_texture = nv12.as_raw() as *mut c_void;
        std::mem::forget(nv12);
        std::mem::forget(src_texture);
        std::mem::forget(device_ptr);
    }

    // 转换 BGRA 到 NV12
    // DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709 = 1
    // DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709 = 2
    if hwcodec_native_device_bgra_to_nv12(
        enc.native_device,
        texture,
        enc.nv12_texture,
        enc.width as u32,
        enc.height as u32,
        1, // RGB_FULL_G22_NONE_P709
        2, // YCBCR_STUDIO_G22_LEFT_P709
    ) == 0 {
        return -1;
    }

    // 3. 设置 Surface 的 MemId
    mfx_wrapper_surface_set_mem_id(surface, enc.nv12_texture);

    // 4. 编码循环
    let start_time = std::time::Instant::now();
    const ENCODE_TIMEOUT_MS: u128 = 1000;
    let mut encoded = false;

    loop {
        if start_time.elapsed().as_millis() > ENCODE_TIMEOUT_MS {
            break;
        }

        // 重置 Bitstream
        mfx_wrapper_bitstream_init(
            enc.bitstream,
            enc.bitstream_data.as_mut_ptr(),
            enc.bitstream_data.len() as u32,
        );

        // 创建 SyncPoint
        let syncp = mfx_wrapper_create_syncpoint();
        if syncp.is_null() {
            break;
        }

        // 提交编码
        let encode_result = mfx_wrapper_encoder_encode_frame_async(
            enc.encoder,
            surface,
            enc.bitstream,
            syncp,
        );

        match encode_result {
            0 => {
                // 成功，同步操作
                if mfx_wrapper_sync_operation(enc.session, syncp, 1000) == 0 {
                    let data_length = mfx_wrapper_bitstream_get_length(enc.bitstream);
                    if data_length > 0 {
                        let data = mfx_wrapper_bitstream_get_data(enc.bitstream);
                        let frame_type = mfx_wrapper_bitstream_get_frame_type(enc.bitstream);
                        let is_keyframe = (frame_type & (MFX_FRAMETYPE_I | MFX_FRAMETYPE_IDR)) != 0;

                        // 调用回调
                        if let Some(cb) = callback {
                            cb(data, data_length as i32, is_keyframe as i32, obj, ms);
                        }
                        encoded = true;
                    }
                }
                mfx_wrapper_destroy_syncpoint(syncp);
                break;
            }
            1 => {
                // 需要更多输入
                mfx_wrapper_destroy_syncpoint(syncp);
                break;
            }
            2 => {
                // 需要更多 Surface
                mfx_wrapper_destroy_syncpoint(syncp);
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            3 => {
                // 设备忙
                mfx_wrapper_destroy_syncpoint(syncp);
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            _ => {
                // 失败
                mfx_wrapper_destroy_syncpoint(syncp);
                break;
            }
        }
    }

    if encoded {
        0
    } else {
        -1
    }
}

#[cfg(not(windows))]
pub unsafe extern "C" fn mfx_encode(
    _encoder: *mut c_void,
    _texture: *mut c_void,
    _callback: EncodeCallback,
    _obj: *mut c_void,
    _ms: i64,
) -> i32 {
    -1
}

// 辅助函数：销毁编码器
#[cfg(windows)]
pub unsafe extern "C" fn mfx_destroy_encoder(encoder: *mut c_void) -> i32 {
    if encoder.is_null() {
        return 0;
    }

    let enc = Box::from_raw(encoder as *mut MfxEncoder);

    // 1. 关闭编码器
    if !enc.encoder.is_null() {
        mfx_wrapper_encoder_close(enc.encoder);
    }

    // 2. 释放 Bitstream
    if !enc.bitstream.is_null() {
        mfx_wrapper_destroy_bitstream(enc.bitstream);
    }

    // 3. 释放 Surface 数组
    if !enc.surfaces_array.is_null() {
        mfx_wrapper_destroy_surface_array(enc.surfaces_array);
    }

    // 4. 释放编码器参数
    if !enc.encoder_params.is_null() {
        mfx_wrapper_destroy_encoder_params(enc.encoder_params);
    }

    // 5. 关闭 Session
    if !enc.session.is_null() {
        mfx_wrapper_session_close(enc.session);
    }

    // 6. 释放 NativeDevice
    if !enc.native_device.is_null() {
        hwcodec_native_device_destroy(enc.native_device);
    }

    0
}

#[cfg(not(windows))]
pub unsafe extern "C" fn mfx_destroy_encoder(_encoder: *mut c_void) -> i32 {
    0
}

// 辅助函数：创建解码器
#[cfg(windows)]
pub unsafe extern "C" fn mfx_new_decoder(
    device: *mut c_void,
    luid: i64,
    data_format: i32,
) -> *mut c_void {
    // 1. 转换编解码器格式
    let codec_id = match convert_codec_to_mfx(data_format) {
        Some(id) => id,
        None => return std::ptr::null_mut(),
    };

    let data_format_enum = match data_format {
        0 => H264,
        1 => H265,
        _ => return std::ptr::null_mut(),
    };

    // 2. 创建 NativeDevice
    extern "C" {
        fn hwcodec_native_device_new(
            luid: i64,
            device: *mut c_void,
            pool_size: i32,
        ) -> *mut c_void;
        fn hwcodec_native_device_get_device(handle: *mut c_void) -> *mut c_void;
    }

    let native_device = if !device.is_null() {
        hwcodec_native_device_new(luid, device, 4)
    } else {
        // 如果 device 为 null，需要先创建设备
        // 暂时返回 null
        return std::ptr::null_mut();
    };

    if native_device.is_null() {
        return std::ptr::null_mut();
    }

    let amf_device = hwcodec_native_device_get_device(native_device);
    if amf_device.is_null() {
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 3. 初始化 MFX Session
    let mut session: *mut c_void = std::ptr::null_mut();
    if mfx_wrapper_session_init(&mut session) != 0 {
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 4. 设置 D3D11 设备句柄
    if mfx_wrapper_session_set_handle_d3d11(session, amf_device) != 0 {
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 5. 创建 D3D11 帧分配器
    let mut allocator: *mut c_void = std::ptr::null_mut();
    if mfx_wrapper_create_d3d11_frame_allocator(amf_device, &mut allocator) != 0 {
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 6. 设置帧分配器
    if mfx_wrapper_session_set_frame_allocator(session, allocator) != 0 {
        mfx_wrapper_allocator_release(allocator);
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 7. 创建解码器
    let mut decoder: *mut c_void = std::ptr::null_mut();
    if mfx_wrapper_create_decoder(session, &mut decoder) != 0 {
        mfx_wrapper_allocator_release(allocator);
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 8. 创建基础解码器参数（后续从 Bitstream 头更新）
    let decoder_params = mfx_wrapper_create_decoder_params(codec_id);
    if decoder_params.is_null() {
        mfx_wrapper_decoder_close(decoder);
        mfx_wrapper_allocator_release(allocator);
        mfx_wrapper_session_close(session);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 9. 创建解码器结构（Surface 数组将在第一次解码时分配）
    let decoder_struct = Box::new(MfxDecoder {
        session,
        decoder,
        decoder_params,
        native_device,
        surfaces_array: std::ptr::null_mut(),
        surface_count: 0,
        allocator,
        initialized: false,
        data_format: data_format_enum,
    });

    Box::into_raw(decoder_struct) as *mut c_void
}

#[cfg(not(windows))]
pub unsafe extern "C" fn mfx_new_decoder(
    _device: *mut c_void,
    _luid: i64,
    _data_format: i32,
) -> *mut c_void {
    std::ptr::null_mut()
}

// 辅助函数：解码
#[cfg(windows)]
pub unsafe extern "C" fn mfx_decode(
    decoder: *mut c_void,
    data: *mut u8,
    length: i32,
    callback: DecodeCallback,
    obj: *mut c_void,
) -> i32 {
    if decoder.is_null() || data.is_null() || length <= 0 {
        return -1;
    }

    let dec = &mut *(decoder as *mut MfxDecoder);

    // 1. 创建 Bitstream
    let bitstream = mfx_wrapper_create_bitstream(length as u32);
    if bitstream.is_null() {
        return -1;
    }
    mfx_wrapper_bitstream_init(bitstream, data, length as u32);

    // 2. 如果未初始化，从 Bitstream 头初始化
    if !dec.initialized {
        // 从 Bitstream 头解码参数
        if mfx_wrapper_decoder_decode_header(dec.decoder, bitstream, dec.decoder_params) != 0 {
            mfx_wrapper_destroy_bitstream(bitstream);
            return -1;
        }

        // 初始化解码器并分配 Surface
        let mut surfaces: *mut c_void = std::ptr::null_mut();
        let mut surface_count: i32 = 0;
        if mfx_wrapper_decoder_initialize_from_bitstream(
            dec.decoder,
            bitstream,
            dec.decoder_params,
            dec.allocator,
            &mut surfaces,
            &mut surface_count,
        ) != 0 {
            mfx_wrapper_destroy_bitstream(bitstream);
            return -1;
        }

        dec.surfaces_array = surfaces;
        dec.surface_count = surface_count;
        dec.initialized = true;
    }

    // 3. 重新设置 Bitstream（因为可能被修改）
    mfx_wrapper_bitstream_init(bitstream, data, length as u32);

    // 4. 解码循环
    let start_time = std::time::Instant::now();
    const DECODE_TIMEOUT_MS: u128 = 1000;
    let mut decoded = false;

    loop {
        if start_time.elapsed().as_millis() > DECODE_TIMEOUT_MS {
            break;
        }

        // 获取空闲 Surface
        let surface_idx = mfx_wrapper_get_free_surface_index(dec.surfaces_array, dec.surface_count);
        if surface_idx < 0 {
            break;
        }

        let surface_work = mfx_wrapper_get_surface_at(dec.surfaces_array, surface_idx);
        if surface_work.is_null() {
            break;
        }

        // 创建输出 Surface 指针和 SyncPoint
        let mut surface_out: *mut c_void = std::ptr::null_mut();
        let syncp = mfx_wrapper_create_syncpoint();
        if syncp.is_null() {
            break;
        }

        // 解码
        let decode_result = mfx_wrapper_decoder_decode_frame_async(
            dec.decoder,
            bitstream,
            surface_work,
            &mut surface_out,
            syncp,
        );

        match decode_result {
            0 => {
                // 成功，同步操作
                if mfx_wrapper_sync_operation(dec.session, syncp, 1000) == 0 {
                    if !surface_out.is_null() {
                        // 转换 Surface 到纹理
                        extern "C" {
                            fn hwcodec_native_device_ensure_texture(
                                handle: *mut c_void,
                                width: u32,
                                height: u32,
                            ) -> i32;
                            fn hwcodec_native_device_next(handle: *mut c_void) -> i32;
                            fn hwcodec_native_device_get_current_texture(handle: *mut c_void) -> *mut c_void;
                            fn hwcodec_native_device_nv12_to_bgra(
                                handle: *mut c_void,
                                nv12_texture: *mut c_void,
                                bgra_texture: *mut c_void,
                                width: u32,
                                height: u32,
                                nv12_array_index: u32,
                            ) -> i32;
                        }

                        // 获取 Surface 信息（宽度和高度）
                        let mut frame_info = std::mem::zeroed::<mfxFrameInfo>();
                        let width: u32;
                        let height: u32;
                        
                        if mfx_wrapper_surface_get_info(surface_out, &mut frame_info as *mut _ as *mut c_void) == 0 {
                            // 从 Surface 获取实际尺寸
                            width = frame_info.crop_w as u32;
                            height = frame_info.crop_h as u32;
                        } else {
                            // 如果获取失败，使用解码器参数中的尺寸（如果有）
                            // 或者使用默认值
                            width = 1920;
                            height = 1080;
                        }

                        // 确保 NativeDevice 纹理
                        if hwcodec_native_device_ensure_texture(
                            dec.native_device,
                            width,
                            height,
                        ) != 0 {
                            mfx_wrapper_destroy_syncpoint(syncp);
                            break;
                        }

                        if hwcodec_native_device_next(dec.native_device) != 0 {
                            mfx_wrapper_destroy_syncpoint(syncp);
                            break;
                        }

                        let dst_texture = hwcodec_native_device_get_current_texture(dec.native_device);
                        if dst_texture.is_null() {
                            mfx_wrapper_destroy_syncpoint(syncp);
                            break;
                        }

                        // 获取 Surface 的 MemId（NV12 纹理）
                        let nv12_texture = mfx_wrapper_surface_get_mem_id(surface_out);
                        if nv12_texture.is_null() {
                            mfx_wrapper_destroy_syncpoint(syncp);
                            break;
                        }

                        // 转换 NV12 到 BGRA
                        // DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709 = 2
                        // DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709 = 1
                        if hwcodec_native_device_nv12_to_bgra(
                            dec.native_device,
                            nv12_texture,
                            dst_texture,
                            width,
                            height,
                            0, // nv12_array_index
                        ) == 0 {
                            mfx_wrapper_destroy_syncpoint(syncp);
                            break;
                        }

                        // 调用回调
                        if let Some(cb) = callback {
                            cb(dst_texture, obj);
                        }

                        decoded = true;
                    }
                }
                mfx_wrapper_destroy_syncpoint(syncp);
                break;
            }
            1 => {
                // 需要更多数据
                mfx_wrapper_destroy_syncpoint(syncp);
                break;
            }
            2 => {
                // 需要更多 Surface
                mfx_wrapper_destroy_syncpoint(syncp);
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            3 => {
                // 设备忙
                mfx_wrapper_destroy_syncpoint(syncp);
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            _ => {
                // 失败或其他错误
                mfx_wrapper_destroy_syncpoint(syncp);
                // 检查是否是不兼容的视频参数（需要重新初始化）
                // 这里简化处理，暂时直接退出
                break;
            }
        }
    }

    mfx_wrapper_destroy_bitstream(bitstream);

    if decoded {
        0
    } else {
        -1
    }
}

#[cfg(not(windows))]
pub unsafe extern "C" fn mfx_decode(
    _decoder: *mut c_void,
    _data: *mut u8,
    _length: i32,
    _callback: DecodeCallback,
    _obj: *mut c_void,
) -> i32 {
    -1
}

// 辅助函数：销毁解码器
#[cfg(windows)]
pub unsafe extern "C" fn mfx_destroy_decoder(decoder: *mut c_void) -> i32 {
    if decoder.is_null() {
        return 0;
    }

    let dec = Box::from_raw(decoder as *mut MfxDecoder);

    // 1. 关闭解码器
    if !dec.decoder.is_null() {
        mfx_wrapper_decoder_close(dec.decoder);
    }

    // 2. 释放 Surface 数组
    if !dec.surfaces_array.is_null() {
        mfx_wrapper_destroy_surface_array(dec.surfaces_array);
    }

    // 3. 释放分配器
    if !dec.allocator.is_null() {
        mfx_wrapper_allocator_release(dec.allocator);
    }

    // 4. 释放解码器参数
    if !dec.decoder_params.is_null() {
        mfx_wrapper_destroy_decoder_params(dec.decoder_params);
    }

    // 5. 关闭 Session
    if !dec.session.is_null() {
        mfx_wrapper_session_close(dec.session);
    }

    // 6. 释放 NativeDevice
    if !dec.native_device.is_null() {
        hwcodec_native_device_destroy(dec.native_device);
    }

    0
}

#[cfg(not(windows))]
pub unsafe extern "C" fn mfx_destroy_decoder(_decoder: *mut c_void) -> i32 {
    0
}

// 辅助函数：测试编码
#[cfg(windows)]
pub unsafe extern "C" fn mfx_test_encode(
    out_luids: *mut i64,
    out_vendors: *mut i32,
    max_desc_num: i32,
    out_desc_num: *mut i32,
    data_format: i32,
    width: i32,
    height: i32,
    kbs: i32,
    framerate: i32,
    gop: i32,
    excluded_luids: *const i64,
    exclude_formats: *const i32,
    exclude_count: i32,
) -> i32 {
    extern "C" {
        fn hwcodec_adapters_new(vendor: i32) -> *mut c_void;
        fn hwcodec_adapters_destroy(handle: *mut c_void);
        fn hwcodec_adapters_get_count(handle: *mut c_void) -> i32;
        fn hwcodec_adapters_get_adapter_luid(handle: *mut c_void, index: i32) -> i64;
        fn hwcodec_adapters_get_adapter_device(handle: *mut c_void, index: i32) -> *mut c_void;
    }

    // ADAPTER_VENDOR_INTEL = 0x8086
    let adapters = hwcodec_adapters_new(0x8086);
    if adapters.is_null() {
        return -1;
    }

    let adapter_count = hwcodec_adapters_get_count(adapters);
    let mut count = 0;

    for i in 0..adapter_count {
        if count >= max_desc_num {
            break;
        }

        let current_luid = hwcodec_adapters_get_adapter_luid(adapters, i);

        // 检查排除列表
        let mut skip = false;
        for j in 0..exclude_count {
            if !excluded_luids.is_null() {
                if *excluded_luids.offset(j as isize) == current_luid {
                    skip = true;
                    break;
                }
            }
            if !exclude_formats.is_null() {
                if *exclude_formats.offset(j as isize) == data_format {
                    skip = true;
                    break;
                }
            }
        }
        if skip {
            continue;
        }

        // 获取设备
        let device = hwcodec_adapters_get_adapter_device(adapters, i);
        if device.is_null() {
            continue;
        }

        // 创建编码器
        let encoder = mfx_new_encoder(device, current_luid, data_format, width, height, kbs, framerate, gop);
        if encoder.is_null() {
            continue;
        }

        // 测试编码
        extern "C" {
            fn hwcodec_native_device_ensure_texture(handle: *mut c_void, width: u32, height: u32) -> i32;
            fn hwcodec_native_device_next(handle: *mut c_void) -> i32;
            fn hwcodec_native_device_get_current_texture(handle: *mut c_void) -> *mut c_void;
        }

        let enc = &*(encoder as *const MfxEncoder);
        let mut key_obj = 0i32;

        if hwcodec_native_device_ensure_texture(enc.native_device, width as u32, height as u32) == 0 {
            if hwcodec_native_device_next(enc.native_device) == 0 {
                let current_texture = hwcodec_native_device_get_current_texture(enc.native_device);
                if !current_texture.is_null() {
                    let start_time = std::time::Instant::now();
                    let succ = mfx_encode(encoder, current_texture, Some(test_encode_callback), &mut key_obj as *mut i32 as *mut c_void, 0) == 0 && key_obj == 1;
                    let elapsed = start_time.elapsed().as_millis();
                    if succ && elapsed < 1000 {
                        if !out_luids.is_null() {
                            *out_luids.offset(count as isize) = current_luid;
                        }
                        if !out_vendors.is_null() {
                            *out_vendors.offset(count as isize) = 2; // VENDOR_INTEL
                        }
                        count += 1;
                    }
                }
            }
        }

        mfx_destroy_encoder(encoder);
    }

    hwcodec_adapters_destroy(adapters);

    if !out_desc_num.is_null() {
        *out_desc_num = count;
    }

    0
}

// 测试编码回调
unsafe extern "C" fn test_encode_callback(
    data: *const u8,
    length: i32,
    key: i32,
    obj: *const c_void,
    _ms: i64,
) {
    if !obj.is_null() {
        let key_ptr = obj as *const i32 as *mut i32;
        *key_ptr = key;
    }
}

#[cfg(not(windows))]
pub unsafe extern "C" fn mfx_test_encode(
    _out_luids: *mut i64,
    _out_vendors: *mut i32,
    _max_desc_num: i32,
    _out_desc_num: *mut i32,
    _data_format: i32,
    _width: i32,
    _height: i32,
    _kbs: i32,
    _framerate: i32,
    _gop: i32,
    _excluded_luids: *const i64,
    _exclude_formats: *const i32,
    _exclude_count: i32,
) -> i32 {
    -1
}

// 辅助函数：测试解码
#[cfg(windows)]
pub unsafe extern "C" fn mfx_test_decode(
    out_luids: *mut i64,
    out_vendors: *mut i32,
    max_desc_num: i32,
    out_desc_num: *mut i32,
    data_format: i32,
    data: *mut u8,
    length: i32,
    excluded_luids: *const i64,
    exclude_formats: *const i32,
    exclude_count: i32,
) -> i32 {
    extern "C" {
        fn hwcodec_adapters_new(vendor: i32) -> *mut c_void;
        fn hwcodec_adapters_destroy(handle: *mut c_void);
        fn hwcodec_adapters_get_count(handle: *mut c_void) -> i32;
        fn hwcodec_adapters_get_adapter_luid(handle: *mut c_void, index: i32) -> i64;
    }

    // ADAPTER_VENDOR_INTEL = 0x8086
    let adapters = hwcodec_adapters_new(0x8086);
    if adapters.is_null() {
        return -1;
    }

    let adapter_count = hwcodec_adapters_get_count(adapters);
    let mut count = 0;

    for i in 0..adapter_count {
        if count >= max_desc_num {
            break;
        }

        let current_luid = hwcodec_adapters_get_adapter_luid(adapters, i);

        // 检查排除列表
        let mut skip = false;
        for j in 0..exclude_count {
            if !excluded_luids.is_null() {
                if *excluded_luids.offset(j as isize) == current_luid {
                    skip = true;
                    break;
                }
            }
            if !exclude_formats.is_null() {
                if *exclude_formats.offset(j as isize) == data_format {
                    skip = true;
                    break;
                }
            }
        }
        if skip {
            continue;
        }

        // 创建解码器（device 为 null，由解码器内部创建）
        let decoder = mfx_new_decoder(std::ptr::null_mut(), current_luid, data_format);
        if decoder.is_null() {
            continue;
        }

        // 测试解码
        let start_time = std::time::Instant::now();
        let succ = mfx_decode(decoder, data, length, None, std::ptr::null_mut()) == 0;
        let elapsed = start_time.elapsed().as_millis();
        if succ && elapsed < 1000 {
            if !out_luids.is_null() {
                *out_luids.offset(count as isize) = current_luid;
            }
            if !out_vendors.is_null() {
                *out_vendors.offset(count as isize) = 2; // VENDOR_INTEL
            }
            count += 1;
        }

        mfx_destroy_decoder(decoder);
    }

    hwcodec_adapters_destroy(adapters);

    if !out_desc_num.is_null() {
        *out_desc_num = count;
    }

    0
}

#[cfg(not(windows))]
pub unsafe extern "C" fn mfx_test_decode(
    _out_luids: *mut i64,
    _out_vendors: *mut i32,
    _max_desc_num: i32,
    _out_desc_num: *mut i32,
    _data_format: i32,
    _data: *mut u8,
    _length: i32,
    _excluded_luids: *const i64,
    _exclude_formats: *const i32,
    _exclude_count: i32,
) -> i32 {
    -1
}

// 辅助函数：设置码率
#[cfg(windows)]
pub unsafe extern "C" fn mfx_set_bitrate(encoder: *mut c_void, kbs: i32) -> i32 {
    if encoder.is_null() {
        return -1;
    }

    let enc = &mut *(encoder as *mut MfxEncoder);

    // 1. 获取当前编码器参数
    if mfx_wrapper_encoder_get_video_param(enc.encoder, enc.encoder_params) != 0 {
        return -1;
    }

    // 2. 更新码率参数
    // 注意：encoder_params 是 mfxVideoParam*，需要在 C 中修改
    // 这里需要创建一个新的参数结构或使用 C 函数修改
    // 暂时使用 Reset 方法，需要重新创建参数
    
    // 重新创建编码器参数
    let codec_id = match convert_codec_to_mfx(enc.data_format as i32) {
        Some(id) => id,
        None => return -1,
    };

    let new_params = mfx_wrapper_create_encoder_params(
        codec_id,
        enc.width,
        enc.height,
        enc.framerate,
        kbs,
        enc.gop,
    );
    if new_params.is_null() {
        return -1;
    }

    // 3. 重置编码器
    if mfx_wrapper_encoder_reset(enc.encoder, new_params) != 0 {
        mfx_wrapper_destroy_encoder_params(new_params);
        return -1;
    }

    // 4. 更新参数指针
    mfx_wrapper_destroy_encoder_params(enc.encoder_params);
    enc.encoder_params = new_params;
    enc.bitrate = kbs * 1000;

    0
}

#[cfg(not(windows))]
pub unsafe extern "C" fn mfx_set_bitrate(_encoder: *mut c_void, _kbs: i32) -> i32 {
    -1
}

// 辅助函数：设置帧率
#[cfg(windows)]
pub unsafe extern "C" fn mfx_set_framerate(encoder: *mut c_void, framerate: i32) -> i32 {
    // MFX 不支持动态改变帧率，返回 -1
    // 参考 C++ 实现：mfx_set_framerate 直接返回 -1
    -1
}

#[cfg(not(windows))]
pub unsafe extern "C" fn mfx_set_framerate(_encoder: *mut c_void, _framerate: i32) -> i32 {
    -1
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
