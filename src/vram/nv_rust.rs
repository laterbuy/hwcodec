//! NVIDIA Video Codec SDK 的 Rust 实现
//! Windows平台使用 C 包装层（nv_wrapper.cpp）调用 SDK，所有业务逻辑在 Rust 中实现

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

// NVIDIA SDK C 包装层 FFI 绑定
#[cfg(windows)]
mod nv_wrapper_ffi {
    use super::*;
    
    extern "C" {
        // 编码器操作
        pub fn nv_wrapper_create_encoder(
            cuda_dl: *mut c_void,
            nvenc_dl: *mut c_void,
            device: *mut c_void,
            width: i32,
            height: i32,
            codec_id: i32,
            bitrate_kbps: i32,
            framerate: i32,
            gop: i32,
            encoder: *mut *mut c_void,
        ) -> i32;
        pub fn nv_wrapper_destroy_encoder(encoder: *mut c_void);
        pub fn nv_wrapper_encoder_get_next_input_frame(encoder: *mut c_void) -> *mut c_void;
        pub fn nv_wrapper_encoder_encode_frame(
            encoder: *mut c_void,
            input_texture: *mut c_void,
            timestamp: i64,
            packet_data: *mut c_void,
            packet_size: *mut u32,
            picture_type: *mut u32,
        ) -> i32;
        pub fn nv_wrapper_encoder_reconfigure(
            encoder: *mut c_void,
            bitrate_kbps: i32,
            framerate: i32,
        ) -> i32;
        
        // 解码器操作
        pub fn nv_wrapper_create_decoder(
            cuda_dl: *mut c_void,
            cuvid_dl: *mut c_void,
            cu_context: *mut c_void,
            codec_id: i32,
            decoder: *mut *mut c_void,
        ) -> i32;
        pub fn nv_wrapper_destroy_decoder(decoder: *mut c_void);
        pub fn nv_wrapper_decoder_decode(
            decoder: *mut c_void,
            data: *const u8,
            length: i32,
            flags: u32,
        ) -> i32;
        pub fn nv_wrapper_decoder_get_frame(decoder: *mut c_void) -> *mut c_void;
        pub fn nv_wrapper_decoder_get_width(decoder: *mut c_void) -> i32;
        pub fn nv_wrapper_decoder_get_height(decoder: *mut c_void) -> i32;
        pub fn nv_wrapper_decoder_get_chroma_height(decoder: *mut c_void) -> i32;
        
        // CUDA 操作
        pub fn nv_wrapper_load_encoder_driver(cuda_dl: *mut *mut c_void, nvenc_dl: *mut *mut c_void) -> i32;
        pub fn nv_wrapper_free_encoder_driver(cuda_dl: *mut *mut c_void, nvenc_dl: *mut *mut c_void);
        pub fn nv_wrapper_load_decoder_driver(cuda_dl: *mut *mut c_void, cuvid_dl: *mut *mut c_void) -> i32;
        pub fn nv_wrapper_free_decoder_driver(cuda_dl: *mut *mut c_void, cuvid_dl: *mut *mut c_void);
        pub fn nv_wrapper_cuda_init(cuda_dl: *mut c_void) -> i32;
        pub fn nv_wrapper_cuda_get_device_from_d3d11(
            cuda_dl: *mut c_void,
            adapter: *mut c_void,
            cu_device: *mut u32,
        ) -> i32;
        pub fn nv_wrapper_cuda_create_context(
            cuda_dl: *mut c_void,
            cu_device: u32,
            cu_context: *mut *mut c_void,
        ) -> i32;
        pub fn nv_wrapper_cuda_destroy_context(cuda_dl: *mut c_void, cu_context: *mut c_void);
        pub fn nv_wrapper_cuda_push_context(cuda_dl: *mut c_void, cu_context: *mut c_void) -> i32;
        pub fn nv_wrapper_cuda_pop_context(cuda_dl: *mut c_void) -> i32;
        
        // 纹理操作
        pub fn nv_wrapper_cuda_register_texture(
            cuda_dl: *mut c_void,
            texture: *mut c_void,
            cu_resource: *mut *mut c_void,
        ) -> i32;
        pub fn nv_wrapper_cuda_unregister_texture(cuda_dl: *mut c_void, cu_resource: *mut c_void);
        pub fn nv_wrapper_cuda_map_resource(cuda_dl: *mut c_void, cu_resource: *mut c_void) -> i32;
        pub fn nv_wrapper_cuda_unmap_resource(cuda_dl: *mut c_void, cu_resource: *mut c_void) -> i32;
        pub fn nv_wrapper_cuda_get_mapped_array(cuda_dl: *mut c_void, cu_resource: *mut c_void) -> *mut c_void;
        pub fn nv_wrapper_cuda_memcpy_device_to_array(
            cuda_dl: *mut c_void,
            dst_array: *mut c_void,
            src_device: *const c_void,
            width: u32,
            height: u32,
            src_pitch: u32,
        ) -> i32;
    }
}

#[cfg(windows)]
use nv_wrapper_ffi::*;

// NativeDevice FFI 函数声明（统一在文件顶部声明，避免重复）
#[cfg(windows)]
extern "C" {
    fn hwcodec_native_device_destroy(handle: *mut c_void);
    fn hwcodec_native_device_get_device(handle: *mut c_void) -> *mut c_void;
    fn hwcodec_native_device_get_context(handle: *mut c_void) -> *mut c_void;
    fn hwcodec_native_device_ensure_texture(handle: *mut c_void, width: u32, height: u32) -> i32;
    fn hwcodec_native_device_next(handle: *mut c_void) -> i32;
    fn hwcodec_native_device_get_current_texture(handle: *mut c_void) -> *mut c_void;
    fn hwcodec_native_device_bgra_to_nv12(
        handle: *mut c_void,
        bgra_texture: *mut c_void,
        nv12_texture: *mut c_void,
        width: u32,
        height: u32,
        color_space_in: u32,
        color_space_out: u32,
    ) -> i32;
    fn hwcodec_native_device_nv12_to_bgra(
        handle: *mut c_void,
        nv12_texture: *mut c_void,
        bgra_texture: *mut c_void,
        width: u32,
        height: u32,
        nv12_array_index: u32,
    ) -> i32;
    fn hwcodec_adapters_new(vendor: i32) -> *mut c_void;
    fn hwcodec_adapters_destroy(handle: *mut c_void);
    fn hwcodec_adapters_get_count(handle: *mut c_void) -> i32;
    fn hwcodec_adapters_get_adapter_luid(handle: *mut c_void, index: i32) -> i64;
    fn hwcodec_adapters_get_adapter_device(handle: *mut c_void, index: i32) -> *mut c_void;
}

// 编码器结构（在 Rust 中管理）
#[cfg(windows)]
pub struct NvencEncoder {
    encoder: *mut c_void,      // NvEncoderD3D11*
    native_device: *mut c_void, // NativeDeviceHandle
    cuda_dl: *mut c_void,      // CudaFunctions*
    nvenc_dl: *mut c_void,     // NvencFunctions*
    data_format: crate::common::DataFormat,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
    packet_buffer: Vec<u8>,    // 编码输出缓冲区
}

// 解码器结构（在 Rust 中管理）
#[cfg(windows)]
pub struct NvdecDecoder {
    decoder: *mut c_void,      // NvDecoder*
    native_device: *mut c_void, // NativeDeviceHandle
    cuda_dl: *mut c_void,      // CudaFunctions*
    cuvid_dl: *mut c_void,     // CuvidFunctions*
    cu_context: *mut c_void,   // CUcontext
    cu_resources: [*mut c_void; 2], // CUgraphicsResource[2]
    textures: [*mut c_void; 2], // ID3D11Texture2D*[2]
    data_format: crate::common::DataFormat,
    width: i32,
    height: i32,
    initialized: bool,
}

// 辅助函数：转换编解码器格式
#[cfg(windows)]
fn convert_codec_to_nv(codec_id: i32) -> Option<i32> {
    match codec_id {
        0 => Some(0),  // H264
        1 => Some(1),  // H265
        _ => None,
    }
}

// 辅助函数：检查NVENC驱动支持
#[cfg(windows)]
pub fn nv_encode_driver_support() -> i32 {
    unsafe {
        let mut cuda_dl: *mut c_void = std::ptr::null_mut();
        let mut nvenc_dl: *mut c_void = std::ptr::null_mut();
        
        if nv_wrapper_load_encoder_driver(&mut cuda_dl, &mut nvenc_dl) != 0 {
            return -1;
        }
        
        nv_wrapper_free_encoder_driver(&mut cuda_dl, &mut nvenc_dl);
        0
    }
}

#[cfg(not(windows))]
pub fn nv_encode_driver_support() -> i32 {
    -1
}

// 辅助函数：检查NVDEC驱动支持
#[cfg(windows)]
pub fn nv_decode_driver_support() -> i32 {
    unsafe {
        let mut cuda_dl: *mut c_void = std::ptr::null_mut();
        let mut cuvid_dl: *mut c_void = std::ptr::null_mut();
        
        if nv_wrapper_load_decoder_driver(&mut cuda_dl, &mut cuvid_dl) != 0 {
            return -1;
        }
        
        nv_wrapper_free_decoder_driver(&mut cuda_dl, &mut cuvid_dl);
        0
    }
}

#[cfg(not(windows))]
pub fn nv_decode_driver_support() -> i32 {
    -1
}

// 辅助函数：创建编码器
#[cfg(windows)]
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
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    // 1. 转换编解码器格式
    let codec_id = match convert_codec_to_nv(data_format) {
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

    // 3. 加载 CUDA 和 NVENC 库
    let mut cuda_dl: *mut c_void = std::ptr::null_mut();
    let mut nvenc_dl: *mut c_void = std::ptr::null_mut();
    if nv_wrapper_load_encoder_driver(&mut cuda_dl, &mut nvenc_dl) != 0 {
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 4. 初始化 CUDA
    if nv_wrapper_cuda_init(cuda_dl) != 0 {
        nv_wrapper_free_encoder_driver(&mut cuda_dl, &mut nvenc_dl);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 5. 获取 CUDA 设备（从 D3D11 适配器）
    use windows::Win32::Graphics::Dxgi::*;
    use windows::core::Interface;
    
    let device_ptr: windows::Win32::Graphics::Direct3D11::ID3D11Device = 
        Interface::from_raw(amf_device as *mut std::ffi::c_void);
    
    let dxgi_device: IDXGIDevice = match Interface::cast(&device_ptr) {
        Ok(dxgi) => dxgi,
        Err(_) => {
            std::mem::forget(device_ptr);
            nv_wrapper_free_encoder_driver(&mut cuda_dl, &mut nvenc_dl);
            hwcodec_native_device_destroy(native_device);
            return std::ptr::null_mut();
        }
    };
    
    let adapter: IDXGIAdapter = match unsafe { dxgi_device.GetAdapter() } {
        Ok(adapter) => adapter,
        Err(_) => {
            std::mem::forget(dxgi_device);
            std::mem::forget(device_ptr);
            nv_wrapper_free_encoder_driver(&mut cuda_dl, &mut nvenc_dl);
            hwcodec_native_device_destroy(native_device);
            return std::ptr::null_mut();
        }
    };
    
    let adapter_ptr = adapter;
    let mut cu_device: u32 = 0;
    if nv_wrapper_cuda_get_device_from_d3d11(
        cuda_dl,
        adapter_ptr.as_raw() as *mut c_void,
        &mut cu_device,
    ) != 0 {
        std::mem::forget(adapter_ptr);
        std::mem::forget(dxgi_device);
        std::mem::forget(device_ptr);
        nv_wrapper_free_encoder_driver(&mut cuda_dl, &mut nvenc_dl);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }
    
    std::mem::forget(adapter_ptr);
    std::mem::forget(dxgi_device);
    std::mem::forget(device_ptr);

    // 6. 创建编码器
    let mut encoder: *mut c_void = std::ptr::null_mut();
    if nv_wrapper_create_encoder(
        cuda_dl,
        nvenc_dl,
        amf_device,
        width,
        height,
        codec_id,
        bitrate,
        framerate,
        gop,
        &mut encoder,
    ) != 0 {
        nv_wrapper_free_encoder_driver(&mut cuda_dl, &mut nvenc_dl);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 7. 创建编码器结构
    let encoder_struct = Box::new(NvencEncoder {
        encoder,
        native_device,
        cuda_dl,
        nvenc_dl,
        data_format: data_format_enum,
        width,
        height,
        bitrate,
        framerate,
        gop,
        packet_buffer: vec![0u8; 1024 * 1024], // 1MB 缓冲区
    });

    Box::into_raw(encoder_struct) as *mut c_void
}

#[cfg(not(windows))]
pub unsafe extern "C" fn nv_new_encoder(
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
pub unsafe extern "C" fn nv_encode(
    encoder: *mut c_void,
    texture: *mut c_void,
    callback: EncodeCallback,
    obj: *mut c_void,
    ms: i64,
) -> i32 {
    if encoder.is_null() || texture.is_null() {
        return -1;
    }

    let enc = &mut *(encoder as *mut NvencEncoder);

    // 1. 获取下一个输入帧
    let input_frame = nv_wrapper_encoder_get_next_input_frame(enc.encoder);
    if input_frame.is_null() {
        return -1;
    }

    // 2. 复制纹理到输入帧（使用 NativeDevice 上下文）
    let context = hwcodec_native_device_get_context(enc.native_device);
    if context.is_null() {
        return -1;
    }
    
    use windows::Win32::Graphics::Direct3D11::*;
    use windows::core::Interface;
    
    let context_ptr: ID3D11DeviceContext = Interface::from_raw(context as *mut std::ffi::c_void);
    let input_tex: ID3D11Texture2D = Interface::from_raw(input_frame as *mut std::ffi::c_void);
    let src_tex: ID3D11Texture2D = Interface::from_raw(texture as *mut std::ffi::c_void);
    
    // 注意：CopyResource 的参数顺序是 (dst, src)
    context_ptr.CopyResource(&input_tex, &src_tex);
    
    std::mem::forget(context_ptr);
    std::mem::forget(input_tex);
    std::mem::forget(src_tex);

    // 3. 编码帧
    let mut packet_size: u32 = enc.packet_buffer.len() as u32;
    let mut picture_type: u32 = 0;
    
    let result = nv_wrapper_encoder_encode_frame(
        enc.encoder,
        texture,
        ms,
        enc.packet_buffer.as_mut_ptr() as *mut c_void,
        &mut packet_size,
        &mut picture_type,
    );
    
    if result != 0 {
        return -1;
    }

    // 4. 调用回调
    if packet_size > 0 {
        let is_keyframe = (picture_type == 1) as i32;
        if let Some(cb) = callback {
            cb(
                enc.packet_buffer.as_ptr(),
                packet_size as i32,
                is_keyframe,
                obj,
                ms,
            );
        }
        return 0;
    }

    -1
}

#[cfg(not(windows))]
pub unsafe extern "C" fn nv_encode(
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
pub unsafe extern "C" fn nv_destroy_encoder(encoder: *mut c_void) -> i32 {
    if encoder.is_null() {
        return 0;
    }

    let enc = Box::from_raw(encoder as *mut NvencEncoder);

    // 1. 销毁编码器
    if !enc.encoder.is_null() {
        nv_wrapper_destroy_encoder(enc.encoder);
    }

    // 2. 释放 CUDA 和 NVENC 库
    let mut cuda_dl = enc.cuda_dl;
    let mut nvenc_dl = enc.nvenc_dl;
    nv_wrapper_free_encoder_driver(&mut cuda_dl, &mut nvenc_dl);

    // 3. 释放 NativeDevice
    if !enc.native_device.is_null() {
        hwcodec_native_device_destroy(enc.native_device);
    }

    0
}

#[cfg(not(windows))]
pub unsafe extern "C" fn nv_destroy_encoder(_encoder: *mut c_void) -> i32 {
    0
}

// 辅助函数：创建解码器
#[cfg(windows)]
pub unsafe extern "C" fn nv_new_decoder(
    device: *mut c_void,
    luid: i64,
    codec_id: i32,
) -> *mut c_void {
    if device.is_null() {
        return std::ptr::null_mut();
    }

    // 1. 转换编解码器格式
    let codec_id_nv = match convert_codec_to_nv(codec_id) {
        Some(id) => id,
        None => return std::ptr::null_mut(),
    };

    let data_format_enum = match codec_id {
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
    }

    let native_device = hwcodec_native_device_new(luid, device, 4);
    if native_device.is_null() {
        return std::ptr::null_mut();
    }

    let d3d_device = hwcodec_native_device_get_device(native_device);
    if d3d_device.is_null() {
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 3. 加载 CUDA 和 NVDEC 库
    let mut cuda_dl: *mut c_void = std::ptr::null_mut();
    let mut cuvid_dl: *mut c_void = std::ptr::null_mut();
    if nv_wrapper_load_decoder_driver(&mut cuda_dl, &mut cuvid_dl) != 0 {
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 4. 初始化 CUDA
    if nv_wrapper_cuda_init(cuda_dl) != 0 {
        nv_wrapper_free_decoder_driver(&mut cuda_dl, &mut cuvid_dl);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 5. 获取 CUDA 设备并创建上下文
    use windows::Win32::Graphics::Dxgi::*;
    use windows::core::Interface;
    
    let device_ptr: windows::Win32::Graphics::Direct3D11::ID3D11Device = 
        Interface::from_raw(d3d_device as *mut std::ffi::c_void);
    
    let dxgi_device: IDXGIDevice = match Interface::cast(&device_ptr) {
        Ok(dxgi) => dxgi,
        Err(_) => {
            std::mem::forget(device_ptr);
            nv_wrapper_free_decoder_driver(&mut cuda_dl, &mut cuvid_dl);
            hwcodec_native_device_destroy(native_device);
            return std::ptr::null_mut();
        }
    };
    
    let adapter: IDXGIAdapter = match unsafe { dxgi_device.GetAdapter() } {
        Ok(adapter) => adapter,
        Err(_) => {
            std::mem::forget(dxgi_device);
            std::mem::forget(device_ptr);
            nv_wrapper_free_decoder_driver(&mut cuda_dl, &mut cuvid_dl);
            hwcodec_native_device_destroy(native_device);
            return std::ptr::null_mut();
        }
    };
    
    let adapter_ptr = adapter;
    let mut cu_device: u32 = 0;
    if nv_wrapper_cuda_get_device_from_d3d11(
        cuda_dl,
        adapter_ptr.as_raw() as *mut c_void,
        &mut cu_device,
    ) != 0 {
        std::mem::forget(adapter_ptr);
        std::mem::forget(dxgi_device);
        std::mem::forget(device_ptr);
        nv_wrapper_free_decoder_driver(&mut cuda_dl, &mut cuvid_dl);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }
    
    std::mem::forget(adapter_ptr);
    std::mem::forget(dxgi_device);
    std::mem::forget(device_ptr);

    // 6. 创建 CUDA 上下文
    let mut cu_context: *mut c_void = std::ptr::null_mut();
    if nv_wrapper_cuda_create_context(cuda_dl, cu_device, &mut cu_context) != 0 {
        nv_wrapper_free_decoder_driver(&mut cuda_dl, &mut cuvid_dl);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 7. 创建解码器
    let mut decoder: *mut c_void = std::ptr::null_mut();
    if nv_wrapper_create_decoder(
        cuda_dl,
        cuvid_dl,
        cu_context,
        codec_id_nv,
        &mut decoder,
    ) != 0 {
        nv_wrapper_cuda_destroy_context(cuda_dl, cu_context);
        nv_wrapper_free_decoder_driver(&mut cuda_dl, &mut cuvid_dl);
        hwcodec_native_device_destroy(native_device);
        return std::ptr::null_mut();
    }

    // 8. 创建解码器结构
    let decoder_struct = Box::new(NvdecDecoder {
        decoder,
        native_device,
        cuda_dl,
        cuvid_dl,
        cu_context,
        cu_resources: [std::ptr::null_mut(); 2],
        textures: [std::ptr::null_mut(); 2],
        data_format: data_format_enum,
        width: 0,
        height: 0,
        initialized: false,
    });

    Box::into_raw(decoder_struct) as *mut c_void
}

#[cfg(not(windows))]
pub unsafe extern "C" fn nv_new_decoder(
    _device: *mut c_void,
    _luid: i64,
    _codec_id: i32,
) -> *mut c_void {
    std::ptr::null_mut()
}

// 辅助函数：解码
#[cfg(windows)]
pub unsafe extern "C" fn nv_decode(
    decoder: *mut c_void,
    data: *mut u8,
    len: i32,
    callback: DecodeCallback,
    obj: *mut c_void,
) -> i32 {
    if decoder.is_null() || data.is_null() || len <= 0 {
        return -1;
    }

    let dec = &mut *(decoder as *mut NvdecDecoder);

    // 1. 解码数据
    const CUVID_PKT_ENDOFPICTURE: u32 = 0x00000001;
    let mut n_frame_returned = nv_wrapper_decoder_decode(dec.decoder, data, len, CUVID_PKT_ENDOFPICTURE);
    
    // 2. 处理解码器重新创建（分辨率变化）
    if n_frame_returned == -2 {
        // 需要重新创建解码器（分辨率改变）
        // 1. 销毁旧解码器
        if !dec.decoder.is_null() {
            nv_wrapper_destroy_decoder(dec.decoder);
            dec.decoder = std::ptr::null_mut();
        }
        
        // 2. 清理旧的 CUDA 资源
        nv_wrapper_cuda_push_context(dec.cuda_dl, dec.cu_context);
        for i in 0..2 {
            if !dec.cu_resources[i].is_null() {
                nv_wrapper_cuda_unmap_resource(dec.cuda_dl, dec.cu_resources[i]);
                nv_wrapper_cuda_unregister_texture(dec.cuda_dl, dec.cu_resources[i]);
                dec.cu_resources[i] = std::ptr::null_mut();
            }
        }
        nv_wrapper_cuda_pop_context(dec.cuda_dl);
        
        // 3. 重置初始化状态
        dec.initialized = false;
        dec.width = 0;
        dec.height = 0;
        
        // 4. 重新创建解码器
        let codec_id_nv = match dec.data_format {
            H264 => 0,
            H265 => 1,
            _ => return -1,
        };
        
        let mut new_decoder: *mut c_void = std::ptr::null_mut();
        if nv_wrapper_create_decoder(
            dec.cuda_dl,
            dec.cuvid_dl,
            dec.cu_context,
            codec_id_nv,
            &mut new_decoder,
        ) != 0 {
            return -1;
        }
        
        dec.decoder = new_decoder;
        
        // 5. 重新解码当前帧
        n_frame_returned = nv_wrapper_decoder_decode(dec.decoder, data, len, CUVID_PKT_ENDOFPICTURE);
        
        if n_frame_returned <= 0 {
            return -1;
        }
    } else if n_frame_returned <= 0 {
        return -1;
    }

    // 2. 获取解码后的尺寸
    let width = nv_wrapper_decoder_get_width(dec.decoder);
    let height = nv_wrapper_decoder_get_height(dec.decoder);
    let chroma_height = nv_wrapper_decoder_get_chroma_height(dec.decoder);
    
    if width <= 0 || height <= 0 {
        return -1;
    }

    // 3. 如果尺寸改变，重新初始化纹理
    if !dec.initialized || dec.width != width || dec.height != height {
        // 清理旧的资源
        if dec.initialized {
            nv_wrapper_cuda_push_context(dec.cuda_dl, dec.cu_context);
            for i in 0..2 {
                if !dec.cu_resources[i].is_null() {
                    nv_wrapper_cuda_unmap_resource(dec.cuda_dl, dec.cu_resources[i]);
                    nv_wrapper_cuda_unregister_texture(dec.cuda_dl, dec.cu_resources[i]);
                }
            }
            nv_wrapper_cuda_pop_context(dec.cuda_dl);
        }

        // 创建新的纹理（R8 和 R8G8）
        extern "C" {
            fn hwcodec_native_device_get_device(handle: *mut c_void) -> *mut c_void;
        }
        
        use windows::Win32::Graphics::Direct3D11::*;
        use windows::Win32::Graphics::Dxgi::Common::*;
        use windows::core::Interface;
        
        let device = hwcodec_native_device_get_device(dec.native_device);
        let device_ptr: ID3D11Device = Interface::from_raw(device as *mut std::ffi::c_void);
        
        // 创建 R8 纹理（Y 平面）
        let desc_y = D3D11_TEXTURE2D_DESC {
            Width: width as u32,
            Height: height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: Default::default(),
            MiscFlags: Default::default(),
        };
        
        let mut texture_y: Option<ID3D11Texture2D> = None;
        if device_ptr.CreateTexture2D(&desc_y, None, Some(&mut texture_y)).is_err() || texture_y.is_none() {
            std::mem::forget(device_ptr);
            return -1;
        }
        let texture_y = texture_y.unwrap();
        dec.textures[0] = texture_y.as_raw() as *mut c_void;
        std::mem::forget(texture_y);
        
        // 创建 R8G8 纹理（UV 平面）
        let desc_uv = D3D11_TEXTURE2D_DESC {
            Width: (width / 2) as u32,
            Height: chroma_height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: Default::default(),
            MiscFlags: Default::default(),
        };
        
        let mut texture_uv: Option<ID3D11Texture2D> = None;
        if device_ptr.CreateTexture2D(&desc_uv, None, Some(&mut texture_uv)).is_err() || texture_uv.is_none() {
            std::mem::forget(device_ptr);
            return -1;
        }
        let texture_uv = texture_uv.unwrap();
        dec.textures[1] = texture_uv.as_raw() as *mut c_void;
        std::mem::forget(texture_uv);
        
        // 注册纹理为 CUDA 资源
        nv_wrapper_cuda_push_context(dec.cuda_dl, dec.cu_context);
        for i in 0..2 {
            if nv_wrapper_cuda_register_texture(dec.cuda_dl, dec.textures[i], &mut dec.cu_resources[i]) != 0 {
                nv_wrapper_cuda_pop_context(dec.cuda_dl);
                std::mem::forget(device_ptr);
                return -1;
            }
        }
        nv_wrapper_cuda_pop_context(dec.cuda_dl);
        
        std::mem::forget(device_ptr);
        dec.width = width;
        dec.height = height;
        dec.initialized = true;
    }

    // 4. 获取解码帧数据
    let frame_data = nv_wrapper_decoder_get_frame(dec.decoder);
    if frame_data.is_null() {
        return -1;
    }

    // 5. 复制 CUDA 帧到纹理
    nv_wrapper_cuda_push_context(dec.cuda_dl, dec.cu_context);
    
    // 映射资源
    for i in 0..2 {
        if nv_wrapper_cuda_map_resource(dec.cuda_dl, dec.cu_resources[i]) != 0 {
            nv_wrapper_cuda_pop_context(dec.cuda_dl);
            return -1;
        }
    }
    
    // 复制 Y 平面
    let dst_array_y = nv_wrapper_cuda_get_mapped_array(dec.cuda_dl, dec.cu_resources[0]);
    if dst_array_y.is_null() {
        for i in 0..2 {
            nv_wrapper_cuda_unmap_resource(dec.cuda_dl, dec.cu_resources[i]);
        }
        nv_wrapper_cuda_pop_context(dec.cuda_dl);
        return -1;
    }
    
    let src_y = frame_data as *const c_void;
    if nv_wrapper_cuda_memcpy_device_to_array(
        dec.cuda_dl,
        dst_array_y,
        src_y,
        width as u32,
        height as u32,
        width as u32,
    ) != 0 {
        for i in 0..2 {
            nv_wrapper_cuda_unmap_resource(dec.cuda_dl, dec.cu_resources[i]);
        }
        nv_wrapper_cuda_pop_context(dec.cuda_dl);
        return -1;
    }
    
    // 复制 UV 平面
    let dst_array_uv = nv_wrapper_cuda_get_mapped_array(dec.cuda_dl, dec.cu_resources[1]);
    if dst_array_uv.is_null() {
        for i in 0..2 {
            nv_wrapper_cuda_unmap_resource(dec.cuda_dl, dec.cu_resources[i]);
        }
        nv_wrapper_cuda_pop_context(dec.cuda_dl);
        return -1;
    }
    
    let src_uv = unsafe { (src_y as *const u8).add((width * height) as usize) as *const c_void };
    if nv_wrapper_cuda_memcpy_device_to_array(
        dec.cuda_dl,
        dst_array_uv,
        src_uv,
        (width / 2) as u32,
        chroma_height as u32,
        width as u32,
    ) != 0 {
        for i in 0..2 {
            nv_wrapper_cuda_unmap_resource(dec.cuda_dl, dec.cu_resources[i]);
        }
        nv_wrapper_cuda_pop_context(dec.cuda_dl);
        return -1;
    }
    
    // 取消映射
    for i in 0..2 {
        nv_wrapper_cuda_unmap_resource(dec.cuda_dl, dec.cu_resources[i]);
    }
    nv_wrapper_cuda_pop_context(dec.cuda_dl);

    // 6. 使用着色器将 R8 和 R8G8 纹理渲染到 BGRA 输出纹理
    // 确保 NativeDevice 纹理并获取
    if hwcodec_native_device_ensure_texture(dec.native_device, width as u32, height as u32) != 0 {
        return -1;
    }

    if hwcodec_native_device_next(dec.native_device) != 0 {
        return -1;
    }

    let dst_texture = hwcodec_native_device_get_current_texture(dec.native_device);
    if dst_texture.is_null() {
        return -1;
    }

    // 执行着色器渲染
    extern "C" {
        fn hwcodec_native_device_get_device(handle: *mut c_void) -> *mut c_void;
        fn hwcodec_native_device_get_context(handle: *mut c_void) -> *mut c_void;
    }
    
    use windows::Win32::Graphics::Direct3D::*;
    use windows::Win32::Graphics::Direct3D11::*;
    use windows::Win32::Graphics::Dxgi::Common::*;
    use windows::core::Interface;
    use crate::platform::win::shader;
    
    let device = hwcodec_native_device_get_device(dec.native_device);
    let context = hwcodec_native_device_get_context(dec.native_device);
    let device_ptr: ID3D11Device = Interface::from_raw(device as *mut std::ffi::c_void);
    let context_ptr: ID3D11DeviceContext = Interface::from_raw(context as *mut std::ffi::c_void);
    
    // 6.1 创建 SRV（Shader Resource View）为 R8 和 R8G8 纹理
    let texture_y: ID3D11Texture2D = Interface::from_raw(dec.textures[0] as *mut std::ffi::c_void);
    let texture_uv: ID3D11Texture2D = Interface::from_raw(dec.textures[1] as *mut std::ffi::c_void);
    
    let srv_desc_y = D3D11_SHADER_RESOURCE_VIEW_DESC {
        Format: DXGI_FORMAT_R8_UNORM,
        ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
        Anonymous: windows::Win32::Graphics::Direct3D11::D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
            Texture2D: D3D11_TEX2D_SRV {
                MostDetailedMip: 0,
                MipLevels: 1,
            },
        },
    };
    
    let mut srv_y: Option<ID3D11ShaderResourceView> = None;
    if device_ptr.CreateShaderResourceView(&texture_y, Some(&srv_desc_y), Some(&mut srv_y)).is_err() || srv_y.is_none() {
        std::mem::forget(texture_y);
        std::mem::forget(texture_uv);
        std::mem::forget(device_ptr);
        std::mem::forget(context_ptr);
        return -1;
    }
    let srv_y = srv_y.unwrap();
    
    let srv_desc_uv = D3D11_SHADER_RESOURCE_VIEW_DESC {
        Format: DXGI_FORMAT_R8G8_UNORM,
        ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
        Anonymous: windows::Win32::Graphics::Direct3D11::D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
            Texture2D: D3D11_TEX2D_SRV {
                MostDetailedMip: 0,
                MipLevels: 1,
            },
        },
    };
    
    let mut srv_uv: Option<ID3D11ShaderResourceView> = None;
    if device_ptr.CreateShaderResourceView(&texture_uv, Some(&srv_desc_uv), Some(&mut srv_uv)).is_err() || srv_uv.is_none() {
        std::mem::forget(srv_y);
        std::mem::forget(texture_y);
        std::mem::forget(texture_uv);
        std::mem::forget(device_ptr);
        std::mem::forget(context_ptr);
        return -1;
    }
    let srv_uv = srv_uv.unwrap();
    
    // 设置 SRV 到像素着色器
    unsafe {
        let srv_ptrs = [Some(srv_y.clone()), Some(srv_uv.clone())];
        context_ptr.PSSetShaderResources(0, Some(&srv_ptrs));
        std::mem::forget(srv_ptrs);
    }
    
    // 6.2 创建 RTV（Render Target View）为 BGRA 输出纹理
    let dst_texture_ptr: ID3D11Texture2D = Interface::from_raw(dst_texture as *mut std::ffi::c_void);
    
    let rt_desc = D3D11_RENDER_TARGET_VIEW_DESC {
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
        Anonymous: windows::Win32::Graphics::Direct3D11::D3D11_RENDER_TARGET_VIEW_DESC_0 {
            Texture2D: D3D11_TEX2D_RTV { MipSlice: 0 },
        },
    };
    
    let mut rtv: Option<ID3D11RenderTargetView> = None;
    if device_ptr.CreateRenderTargetView(&dst_texture_ptr, Some(&rt_desc), Some(&mut rtv)).is_err() || rtv.is_none() {
        std::mem::forget(srv_y);
        std::mem::forget(srv_uv);
        std::mem::forget(texture_y);
        std::mem::forget(texture_uv);
        std::mem::forget(dst_texture_ptr);
        std::mem::forget(device_ptr);
        std::mem::forget(context_ptr);
        return -1;
    }
    let rtv = rtv.unwrap();
    
    // 清除渲染目标并设置
    unsafe {
        let clear_color = [0.0f32, 0.0f32, 0.0f32, 0.0f32];
        context_ptr.ClearRenderTargetView(&rtv, &clear_color);
        let rtv_ptrs = [Some(rtv.clone())];
        context_ptr.OMSetRenderTargets(Some(&rtv_ptrs), None);
        std::mem::forget(rtv_ptrs);
    }
    
    // 6.3 设置视口
    let viewport = D3D11_VIEWPORT {
        TopLeftX: 0.0,
        TopLeftY: 0.0,
        Width: width as f32,
        Height: height as f32,
        MinDepth: 0.0,
        MaxDepth: 1.0,
    };
    unsafe {
        context_ptr.RSSetViewports(Some(&[viewport]));
    }
    
    // 6.4 设置采样器
    let sampler_desc = D3D11_SAMPLER_DESC {
        Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
        AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
        AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
        AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
        MipLODBias: 0.0,
        MaxAnisotropy: 1,
        ComparisonFunc: D3D11_COMPARISON_NEVER,
        BorderColor: [0.0f32; 4],
        MinLOD: 0.0,
        MaxLOD: f32::MAX,
    };
    
    let mut sampler: Option<ID3D11SamplerState> = None;
    if device_ptr.CreateSamplerState(&sampler_desc, Some(&mut sampler)).is_err() || sampler.is_none() {
        std::mem::forget(srv_y);
        std::mem::forget(srv_uv);
        std::mem::forget(rtv);
        std::mem::forget(texture_y);
        std::mem::forget(texture_uv);
        std::mem::forget(dst_texture_ptr);
        std::mem::forget(device_ptr);
        std::mem::forget(context_ptr);
        return -1;
    }
    let sampler = sampler.unwrap();
    
    unsafe {
        let sampler_ptrs = [Some(sampler.clone())];
        context_ptr.PSSetSamplers(0, Some(&sampler_ptrs));
        std::mem::forget(sampler_ptrs);
    }
    
    // 6.5 设置着色器
    let vertex_shader = match shader::create_vertex_shader(&device_ptr) {
        Ok(s) => s,
        Err(_) => {
            std::mem::forget(srv_y);
            std::mem::forget(srv_uv);
            std::mem::forget(rtv);
            std::mem::forget(sampler);
            std::mem::forget(texture_y);
            std::mem::forget(texture_uv);
            std::mem::forget(dst_texture_ptr);
            std::mem::forget(device_ptr);
            std::mem::forget(context_ptr);
            return -1;
        }
    };
    
    let pixel_shader = match shader::create_pixel_shader(&device_ptr) {
        Ok(s) => s,
        Err(_) => {
            std::mem::forget(vertex_shader);
            std::mem::forget(srv_y);
            std::mem::forget(srv_uv);
            std::mem::forget(rtv);
            std::mem::forget(sampler);
            std::mem::forget(texture_y);
            std::mem::forget(texture_uv);
            std::mem::forget(dst_texture_ptr);
            std::mem::forget(device_ptr);
            std::mem::forget(context_ptr);
            return -1;
        }
    };
    
    let input_layout = match shader::create_input_layout(&device_ptr) {
        Ok(l) => l,
        Err(_) => {
            std::mem::forget(vertex_shader);
            std::mem::forget(pixel_shader);
            std::mem::forget(srv_y);
            std::mem::forget(srv_uv);
            std::mem::forget(rtv);
            std::mem::forget(sampler);
            std::mem::forget(texture_y);
            std::mem::forget(texture_uv);
            std::mem::forget(dst_texture_ptr);
            std::mem::forget(device_ptr);
            std::mem::forget(context_ptr);
            return -1;
        }
    };
    
    unsafe {
        context_ptr.IASetInputLayout(Some(&input_layout));
        context_ptr.VSSetShader(Some(&vertex_shader), None);
        context_ptr.PSSetShader(Some(&pixel_shader), None);
        std::mem::forget(input_layout);
        std::mem::forget(vertex_shader);
        std::mem::forget(pixel_shader);
    }
    
    // 6.6 设置混合状态和图元拓扑
    unsafe {
        let blend_factor = [0.0f32; 4];
        context_ptr.OMSetBlendState(None, Some(&blend_factor), 0xffffffff);
        context_ptr.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    }
    
    // 6.7 创建并设置顶点缓冲区
    #[repr(C)]
    struct Vertex {
        pos: [f32; 3],
        tex: [f32; 2],
    }
    
    const NUM_VERTICES: usize = 6;
    let vertices = [
        Vertex { pos: [-1.0, -1.0, 0.0], tex: [0.0, 1.0] },
        Vertex { pos: [-1.0,  1.0, 0.0], tex: [0.0, 0.0] },
        Vertex { pos: [ 1.0, -1.0, 0.0], tex: [1.0, 1.0] },
        Vertex { pos: [ 1.0, -1.0, 0.0], tex: [1.0, 1.0] },
        Vertex { pos: [-1.0,  1.0, 0.0], tex: [0.0, 0.0] },
        Vertex { pos: [ 1.0,  1.0, 0.0], tex: [1.0, 0.0] },
    ];
    
    let buffer_desc = D3D11_BUFFER_DESC {
        ByteWidth: (std::mem::size_of::<Vertex>() * NUM_VERTICES) as u32,
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
        CPUAccessFlags: Default::default(),
        MiscFlags: Default::default(),
        StructureByteStride: 0,
    };
    
    let subresource_data = D3D11_SUBRESOURCE_DATA {
        pSysMem: vertices.as_ptr() as *const std::ffi::c_void,
        SysMemPitch: 0,
        SysMemSlicePitch: 0,
    };
    
    let mut vertex_buffer: Option<ID3D11Buffer> = None;
    if device_ptr.CreateBuffer(&buffer_desc, Some(&subresource_data), Some(&mut vertex_buffer)).is_err() || vertex_buffer.is_none() {
        std::mem::forget(srv_y);
        std::mem::forget(srv_uv);
        std::mem::forget(rtv);
        std::mem::forget(sampler);
        std::mem::forget(texture_y);
        std::mem::forget(texture_uv);
        std::mem::forget(dst_texture_ptr);
        std::mem::forget(device_ptr);
        std::mem::forget(context_ptr);
        return -1;
    }
    let vertex_buffer = vertex_buffer.unwrap();
    
    unsafe {
        let stride = std::mem::size_of::<Vertex>() as u32;
        let offset = 0u32;
        let buffers = [Some(vertex_buffer.clone())];
        let strides = [stride];
        let offsets = [offset];
        context_ptr.IASetVertexBuffers(0, 1, Some(buffers.as_ptr()), Some(strides.as_ptr()), Some(offsets.as_ptr()));
        std::mem::forget(vertex_buffer);
    }
    
    // 6.8 执行绘制
    unsafe {
        context_ptr.Draw(NUM_VERTICES as u32, 0);
        context_ptr.Flush();
    }
    
    // 清理临时资源（SRV、RTV、采样器等会在作用域结束时自动释放）
    std::mem::forget(srv_y);
    std::mem::forget(srv_uv);
    std::mem::forget(rtv);
    std::mem::forget(sampler);
    std::mem::forget(texture_y);
    std::mem::forget(texture_uv);
    std::mem::forget(dst_texture_ptr);
    std::mem::forget(device_ptr);
    std::mem::forget(context_ptr);

    // 7. 调用回调
    if let Some(cb) = callback {
        cb(dst_texture, obj);
    }

    0
}

#[cfg(not(windows))]
pub unsafe extern "C" fn nv_decode(
    _decoder: *mut c_void,
    _data: *mut u8,
    _len: i32,
    _callback: DecodeCallback,
    _obj: *mut c_void,
) -> i32 {
    -1
}

// 辅助函数：销毁解码器
#[cfg(windows)]
pub unsafe extern "C" fn nv_destroy_decoder(decoder: *mut c_void) -> i32 {
    if decoder.is_null() {
        return 0;
    }

    let dec = Box::from_raw(decoder as *mut NvdecDecoder);

    // 1. 销毁解码器
    if !dec.decoder.is_null() {
        nv_wrapper_destroy_decoder(dec.decoder);
    }

    // 2. 清理 CUDA 资源
    if !dec.cuda_dl.is_null() && !dec.cu_context.is_null() {
        nv_wrapper_cuda_push_context(dec.cuda_dl, dec.cu_context);
        for i in 0..2 {
            if !dec.cu_resources[i].is_null() {
                nv_wrapper_cuda_unregister_texture(dec.cuda_dl, dec.cu_resources[i]);
            }
        }
        nv_wrapper_cuda_pop_context(dec.cuda_dl);
        nv_wrapper_cuda_destroy_context(dec.cuda_dl, dec.cu_context);
    }

    // 3. 释放 CUDA 和 NVDEC 库
    let mut cuda_dl = dec.cuda_dl;
    let mut cuvid_dl = dec.cuvid_dl;
    nv_wrapper_free_decoder_driver(&mut cuda_dl, &mut cuvid_dl);

    // 4. 释放 NativeDevice
    if !dec.native_device.is_null() {
        hwcodec_native_device_destroy(dec.native_device);
    }

    0
}

#[cfg(not(windows))]
pub unsafe extern "C" fn nv_destroy_decoder(_decoder: *mut c_void) -> i32 {
    0
}

// 辅助函数：测试编码
#[cfg(windows)]
pub unsafe extern "C" fn nv_test_encode(
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
    // ADAPTER_VENDOR_NVIDIA = 0x10DE
    let adapters = hwcodec_adapters_new(0x10DE);
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
        let encoder = nv_new_encoder(device, current_luid, data_format, width, height, kbs, framerate, gop);
        if encoder.is_null() {
            continue;
        }

        // 测试编码
        if hwcodec_native_device_ensure_texture(
            (*(encoder as *const NvencEncoder)).native_device,
            width as u32,
            height as u32,
        ) == 0 {
            if hwcodec_native_device_next((*(encoder as *const NvencEncoder)).native_device) == 0 {
                let current_texture = hwcodec_native_device_get_current_texture(
                    (*(encoder as *const NvencEncoder)).native_device,
                );
                if !current_texture.is_null() {
                    let mut key_obj = 0i32;
                    let start_time = std::time::Instant::now();
                    let succ = nv_encode(
                        encoder,
                        current_texture,
                        Some(test_encode_callback),
                        &mut key_obj as *mut i32 as *mut c_void,
                        0,
                    ) == 0
                        && key_obj == 1;
                    let elapsed = start_time.elapsed().as_millis();
                    if succ && elapsed < 1000 {
                        if !out_luids.is_null() {
                            *out_luids.offset(count as isize) = current_luid;
                        }
                        if !out_vendors.is_null() {
                            *out_vendors.offset(count as isize) = 0; // VENDOR_NV
                        }
                        count += 1;
                    }
                }
            }
        }

        nv_destroy_encoder(encoder);
    }

    hwcodec_adapters_destroy(adapters);

    if !out_desc_num.is_null() {
        *out_desc_num = count;
    }

    0
}

#[cfg(not(windows))]
pub unsafe extern "C" fn nv_test_encode(
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

// 辅助函数：测试解码
#[cfg(windows)]
pub unsafe extern "C" fn nv_test_decode(
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
    // ADAPTER_VENDOR_NVIDIA = 0x10DE
    let adapters = hwcodec_adapters_new(0x10DE);
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

        // 创建解码器
        let decoder = nv_new_decoder(device, current_luid, data_format);
        if decoder.is_null() {
            continue;
        }

        // 测试解码
        let start_time = std::time::Instant::now();
        let succ = nv_decode(decoder, data, length, None, std::ptr::null_mut()) == 0;
        let elapsed = start_time.elapsed().as_millis();
        if succ && elapsed < 1000 {
            if !out_luids.is_null() {
                *out_luids.offset(count as isize) = current_luid;
            }
            if !out_vendors.is_null() {
                *out_vendors.offset(count as isize) = 0; // VENDOR_NV
            }
            count += 1;
        }

        nv_destroy_decoder(decoder);
    }

    hwcodec_adapters_destroy(adapters);

    if !out_desc_num.is_null() {
        *out_desc_num = count;
    }

    0
}

#[cfg(not(windows))]
pub unsafe extern "C" fn nv_test_decode(
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
pub unsafe extern "C" fn nv_set_bitrate(encoder: *mut c_void, kbs: i32) -> i32 {
    if encoder.is_null() {
        return -1;
    }

    let enc = &*(encoder as *const NvencEncoder);
    if nv_wrapper_encoder_reconfigure(enc.encoder, kbs, -1) == 0 {
        // 更新结构体中的码率
        let enc_mut = &mut *(encoder as *mut NvencEncoder);
        enc_mut.bitrate = kbs;
        return 0;
    }
    -1
}

#[cfg(not(windows))]
pub unsafe extern "C" fn nv_set_bitrate(_encoder: *mut c_void, _kbs: i32) -> i32 {
    -1
}

// 辅助函数：设置帧率
#[cfg(windows)]
pub unsafe extern "C" fn nv_set_framerate(encoder: *mut c_void, framerate: i32) -> i32 {
    if encoder.is_null() {
        return -1;
    }

    let enc = &*(encoder as *const NvencEncoder);
    if nv_wrapper_encoder_reconfigure(enc.encoder, -1, framerate) == 0 {
        // 更新结构体中的帧率
        let enc_mut = &mut *(encoder as *mut NvencEncoder);
        enc_mut.framerate = framerate;
        return 0;
    }
    -1
}

#[cfg(not(windows))]
pub unsafe extern "C" fn nv_set_framerate(_encoder: *mut c_void, _framerate: i32) -> i32 {
    -1
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
    if nv_decode_driver_support() != 0 {
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
