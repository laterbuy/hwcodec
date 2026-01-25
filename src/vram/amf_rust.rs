//! AMF SDK 的 Rust 实现
//! 使用 C 包装层（amf_wrapper.cpp）调用 SDK，所有业务逻辑在 Rust 中实现

use crate::{
    common::DataFormat::*,
    vram::inner::{DecodeCalls, EncodeCalls, InnerDecodeContext, InnerEncodeContext},
};
use serde_derive::{Deserialize, Serialize};
use std::ffi::{c_char, c_int, c_void};

// 从common.h导入的类型
include!(concat!(env!("OUT_DIR"), "/common_ffi.rs"));

// AMF C 包装层 FFI 绑定
#[cfg(windows)]
mod amf_wrapper_ffi {
    use std::os::raw::{c_char, c_int};
    use std::ffi::c_void;
    
    extern "C" {
        // Factory 操作
        pub fn amf_wrapper_factory_init(factory: *mut *mut c_void) -> c_int;
        pub fn amf_wrapper_factory_terminate(factory: *mut c_void);
        pub fn amf_wrapper_create_context(factory: *mut c_void, context: *mut *mut c_void) -> c_int;
        pub fn amf_wrapper_context_init_dx11(context: *mut c_void, device: *mut c_void) -> c_int;
        
        // Component 操作
        pub fn amf_wrapper_create_encoder_component(
            factory: *mut c_void,
            context: *mut c_void,
            codec_name: *const c_char,
            component: *mut *mut c_void,
        ) -> c_int;
        pub fn amf_wrapper_create_decoder_component(
            factory: *mut c_void,
            context: *mut c_void,
            codec_name: *const c_char,
            component: *mut *mut c_void,
        ) -> c_int;
        pub fn amf_wrapper_create_converter_component(
            factory: *mut c_void,
            context: *mut c_void,
            component: *mut *mut c_void,
        ) -> c_int;
        
        // 属性设置
        pub fn amf_wrapper_component_set_property_int64(
            component: *mut c_void,
            name: *const c_char,
            value: i64,
        ) -> c_int;
        pub fn amf_wrapper_component_set_property_int32(
            component: *mut c_void,
            name: *const c_char,
            value: i32,
        ) -> c_int;
        pub fn amf_wrapper_component_set_property_bool(
            component: *mut c_void,
            name: *const c_char,
            value: i32,
        ) -> c_int;
        pub fn amf_wrapper_component_set_property_double(
            component: *mut c_void,
            name: *const c_char,
            value: f64,
        ) -> c_int;
        
        // 组件初始化
        pub fn amf_wrapper_component_init(
            component: *mut c_void,
            format: i32,
            width: i32,
            height: i32,
        ) -> c_int;
        pub fn amf_wrapper_component_terminate(component: *mut c_void);
        pub fn amf_wrapper_component_drain(component: *mut c_void) -> c_int;
        
        // Surface 操作
        pub fn amf_wrapper_create_surface_from_dx11(
            context: *mut c_void,
            texture: *mut c_void,
            surface: *mut *mut c_void,
        ) -> c_int;
        pub fn amf_wrapper_alloc_surface(
            context: *mut c_void,
            memory_type: i32,
            format: i32,
            width: i32,
            height: i32,
            surface: *mut *mut c_void,
        ) -> c_int;
        pub fn amf_wrapper_surface_set_pts(surface: *mut c_void, pts: i64);
        pub fn amf_wrapper_surface_duplicate(
            surface: *mut c_void,
            memory_type: i32,
            new_surface: *mut *mut c_void,
        ) -> c_int;
        
        // 编码器操作
        pub fn amf_wrapper_encoder_submit_input(
            encoder: *mut c_void,
            surface: *mut c_void,
        ) -> c_int;
        pub fn amf_wrapper_encoder_query_output(
            encoder: *mut c_void,
            data: *mut *mut c_void,
        ) -> c_int;
        
        // 解码器操作
        pub fn amf_wrapper_decoder_submit_input(
            decoder: *mut c_void,
            data: *const u8,
            size: i32,
            pts: i64,
        ) -> c_int; // 返回: 0=成功, -1=失败, 2=分辨率变化
        pub fn amf_wrapper_create_buffer_from_host(
            context: *mut c_void,
            data: *const u8,
            size: i32,
            buffer: *mut *mut c_void,
        ) -> c_int;
        pub fn amf_wrapper_decoder_query_output(
            decoder: *mut c_void,
            surface: *mut *mut c_void,
        ) -> c_int;
        
        // 转换器操作
        pub fn amf_wrapper_converter_submit_input(
            converter: *mut c_void,
            surface: *mut c_void,
        ) -> c_int;
        pub fn amf_wrapper_converter_query_output(
            converter: *mut c_void,
            data: *mut *mut c_void,
        ) -> c_int;
        
        // Buffer 操作
        pub fn amf_wrapper_buffer_get_size(buffer: *mut c_void) -> i32;
        pub fn amf_wrapper_buffer_get_native(buffer: *mut c_void) -> *mut c_void;
        pub fn amf_wrapper_buffer_get_property_int64(
            buffer: *mut c_void,
            name: *const c_char,
            value: *mut i64,
        ) -> c_int;
        
        // Surface 查询操作
        pub fn amf_wrapper_surface_get_format(surface: *mut c_void) -> i32;
        pub fn amf_wrapper_surface_get_width(surface: *mut c_void) -> i32;
        pub fn amf_wrapper_surface_get_height(surface: *mut c_void) -> i32;
        pub fn amf_wrapper_surface_get_planes_count(surface: *mut c_void) -> i32;
        pub fn amf_wrapper_surface_get_plane_at(
            surface: *mut c_void,
            plane_index: i32,
        ) -> *mut c_void;
        pub fn amf_wrapper_plane_get_native(plane: *mut c_void) -> *mut c_void;
        
        // 资源释放
        pub fn amf_wrapper_release(ptr: *mut c_void);
    }
}

#[cfg(windows)]
use amf_wrapper_ffi::*;

// AMF SDK 函数指针类型
type AMFInitFn = unsafe extern "system" fn(u64, *mut *mut c_void) -> i32;

// 编码器结构（在 Rust 中管理）
pub struct AmfEncoder {
    factory: *mut c_void,
    context: *mut c_void,
    encoder: *mut c_void,
    converter: *mut c_void,
    device: *mut c_void, // ID3D11Device*
    data_format: crate::common::DataFormat,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
    packet_buffer: Vec<u8>,
}

// 解码器结构（在 Rust 中管理）
pub struct AmfDecoder {
    factory: *mut c_void,
    context: *mut c_void,
    decoder: *mut c_void,
    converter: *mut c_void,
    device: *mut c_void,
    native_device: *mut c_void, // NativeDeviceHandle
    luid: i64,
    data_format: crate::common::DataFormat,
    last_width: i32,
    last_height: i32,
}

// 辅助函数：检查AMF驱动支持
pub fn amf_driver_support() -> i32 {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{PCSTR, PCWSTR};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    
    unsafe {
        let dll_name: Vec<u16> = OsStr::new("amfrt64.dll")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let hmodule = LoadLibraryW(PCWSTR::from_raw(dll_name.as_ptr()));
        if hmodule.is_err() || hmodule.as_ref().unwrap().is_invalid() {
            return -1;
        }
        
        let hmodule = hmodule.unwrap();
        let init_name = b"AMFInit\0";
        let init_fn = GetProcAddress(hmodule, PCSTR::from_raw(init_name.as_ptr()));
        
        if init_fn.is_none() {
            return -1;
        }
        
        type AMFInitFn = unsafe extern "system" fn(u64, *mut *mut c_void) -> i32;
        let init_fn: AMFInitFn = std::mem::transmute(init_fn.unwrap());
        
        let mut factory: *mut c_void = std::ptr::null_mut();
        let result = init_fn(0x00040000, &mut factory);
        
        if result == 0 && !factory.is_null() {
            return 0;
        }
        
        -1
    }
}

// AMF 常量定义（从 AMF SDK 头文件）
const AMF_MEMORY_DX11: i32 = 2;
const AMF_MEMORY_HOST: i32 = 1;
const AMF_SURFACE_BGRA: i32 = 20;
const AMF_SURFACE_NV12: i32 = 7;
const AMF_REPEAT: i32 = 10; // AMF_REPEAT = 10
const AMF_RESOLUTION_CHANGED: i32 = 2; // 从 C 包装层返回的错误码

// DECODE_TIMEOUT_MS 已在 common_ffi.rs 中定义

// H264 编码器属性
const AMF_VIDEO_ENCODER_USAGE: &str = "AMF_VIDEO_ENCODER_USAGE";
const AMF_VIDEO_ENCODER_USAGE_LOW_LATENCY: i64 = 0;
const AMF_VIDEO_ENCODER_FRAMESIZE: &str = "AMF_VIDEO_ENCODER_FRAMESIZE";
const AMF_VIDEO_ENCODER_LOWLATENCY_MODE: &str = "AMF_VIDEO_ENCODER_LOWLATENCY_MODE";
const AMF_VIDEO_ENCODER_QUALITY_PRESET: &str = "AMF_VIDEO_ENCODER_QUALITY_PRESET";
const AMF_VIDEO_ENCODER_QUALITY_PRESET_QUALITY: i64 = 0;
const AMF_VIDEO_ENCODER_COLOR_BIT_DEPTH: &str = "AMF_VIDEO_ENCODER_COLOR_BIT_DEPTH";
const AMF_COLOR_BIT_DEPTH_8: i64 = 0;
const AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD: &str = "AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD";
const AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CBR: i64 = 1;
const AMF_VIDEO_ENCODER_PROFILE: &str = "AMF_VIDEO_ENCODER_PROFILE";
const AMF_VIDEO_ENCODER_PROFILE_HIGH: i64 = 100;
const AMF_VIDEO_ENCODER_PROFILE_LEVEL: &str = "AMF_VIDEO_ENCODER_PROFILE_LEVEL";
const AMF_H264_LEVEL_5_1: i64 = 51;
const AMF_VIDEO_ENCODER_FULL_RANGE_COLOR: &str = "AMF_VIDEO_ENCODER_FULL_RANGE_COLOR";
const AMF_VIDEO_ENCODER_B_PIC_PATTERN: &str = "AMF_VIDEO_ENCODER_B_PIC_PATTERN";
const AMF_VIDEO_ENCODER_QUERY_TIMEOUT: &str = "AMF_VIDEO_ENCODER_QUERY_TIMEOUT";
const AMF_VIDEO_ENCODER_TARGET_BITRATE: &str = "AMF_VIDEO_ENCODER_TARGET_BITRATE";
const AMF_VIDEO_ENCODER_FRAMERATE: &str = "AMF_VIDEO_ENCODER_FRAMERATE";
const AMF_VIDEO_ENCODER_IDR_PERIOD: &str = "AMF_VIDEO_ENCODER_IDR_PERIOD";

// HEVC 编码器属性
const AMF_VIDEO_ENCODER_HEVC_USAGE: &str = "AMF_VIDEO_ENCODER_HEVC_USAGE";
const AMF_VIDEO_ENCODER_HEVC_USAGE_LOW_LATENCY: i64 = 0;
const AMF_VIDEO_ENCODER_HEVC_FRAMESIZE: &str = "AMF_VIDEO_ENCODER_HEVC_FRAMESIZE";
const AMF_VIDEO_ENCODER_HEVC_LOWLATENCY_MODE: &str = "AMF_VIDEO_ENCODER_HEVC_LOWLATENCY_MODE";
const AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET: &str = "AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET";
const AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_QUALITY: i64 = 0;
const AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH: &str = "AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH";
const AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD: &str = "AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD";
const AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_CBR: i64 = 1;
const AMF_VIDEO_ENCODER_HEVC_TIER: &str = "AMF_VIDEO_ENCODER_HEVC_TIER";
const AMF_VIDEO_ENCODER_HEVC_TIER_HIGH: i64 = 1;
const AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL: &str = "AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL";
const AMF_LEVEL_5_1: i64 = 51;
const AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE: &str = "AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE";
const AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE_FULL: i64 = 0;
const AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE_STUDIO: i64 = 1;
const AMF_VIDEO_ENCODER_HEVC_QUERY_TIMEOUT: &str = "AMF_VIDEO_ENCODER_HEVC_QUERY_TIMEOUT";
const AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE: &str = "AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE";
const AMF_VIDEO_ENCODER_HEVC_FRAMERATE: &str = "AMF_VIDEO_ENCODER_HEVC_FRAMERATE";
const AMF_VIDEO_ENCODER_HEVC_GOP_SIZE: &str = "AMF_VIDEO_ENCODER_HEVC_GOP_SIZE";

// 转换器属性
const AMF_VIDEO_CONVERTER_MEMORY_TYPE: &str = "AMF_VIDEO_CONVERTER_MEMORY_TYPE";
const AMF_VIDEO_CONVERTER_OUTPUT_FORMAT: &str = "AMF_VIDEO_CONVERTER_OUTPUT_FORMAT";
const AMF_VIDEO_CONVERTER_OUTPUT_SIZE: &str = "AMF_VIDEO_CONVERTER_OUTPUT_SIZE";

// 编码器组件名称
const AMFVideoEncoderVCE_AVC: &str = "AMFVideoEncoderVCE_AVC";
const AMFVideoEncoder_HEVC: &str = "AMFVideoEncoder_HEVC";
const AMFVideoConverter: &str = "AMFVideoConverter";

// 辅助函数：将 DataFormat 转换为编码器名称
fn get_codec_name(data_format: i32) -> Option<&'static str> {
    match data_format {
        0 => Some(AMFVideoEncoderVCE_AVC), // H264
        1 => Some(AMFVideoEncoder_HEVC),   // H265
        _ => None,
    }
}

// 辅助函数：设置 H264 编码器参数
#[cfg(windows)]
unsafe fn set_h264_params(
    encoder: *mut c_void,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
    enable_4k: bool,
) -> i32 {
    // Usage (可选，不检查错误)
    let _ = amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_USAGE.as_ptr() as *const i8,
        AMF_VIDEO_ENCODER_USAGE_LOW_LATENCY,
    );

    // Frame size (使用 AMFConstructSize，这里简化为 width << 32 | height)
    let frame_size = ((width as i64) << 32) | (height as i64);
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_FRAMESIZE.as_ptr() as *const i8,
        frame_size,
    ) != 0
    {
        return -1;
    }

    // Low latency mode
    if amf_wrapper_component_set_property_bool(
        encoder,
        AMF_VIDEO_ENCODER_LOWLATENCY_MODE.as_ptr() as *const i8,
        1,
    ) != 0
    {
        return -1;
    }

    // Quality preset
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_QUALITY_PRESET.as_ptr() as *const i8,
        AMF_VIDEO_ENCODER_QUALITY_PRESET_QUALITY,
    ) != 0
    {
        return -1;
    }

    // Color bit depth (可选)
    let _ = amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_COLOR_BIT_DEPTH.as_ptr() as *const i8,
        AMF_COLOR_BIT_DEPTH_8,
    );

    // Rate control method
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD.as_ptr() as *const i8,
        AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CBR,
    ) != 0
    {
        return -1;
    }

    // 4K profile settings
    if enable_4k {
        if amf_wrapper_component_set_property_int64(
            encoder,
            AMF_VIDEO_ENCODER_PROFILE.as_ptr() as *const i8,
            AMF_VIDEO_ENCODER_PROFILE_HIGH,
        ) != 0
        {
            return -1;
        }

        if amf_wrapper_component_set_property_int64(
            encoder,
            AMF_VIDEO_ENCODER_PROFILE_LEVEL.as_ptr() as *const i8,
            AMF_H264_LEVEL_5_1,
        ) != 0
        {
            return -1;
        }
    }

    // Full range color (默认 false)
    if amf_wrapper_component_set_property_bool(
        encoder,
        AMF_VIDEO_ENCODER_FULL_RANGE_COLOR.as_ptr() as *const i8,
        0,
    ) != 0
    {
        return -1;
    }

    // B-pic pattern (可选，不检查错误)
    let _ = amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_B_PIC_PATTERN.as_ptr() as *const i8,
        0,
    );

    // Query timeout (1000ms)
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_QUERY_TIMEOUT.as_ptr() as *const i8,
        1000,
    ) != 0
    {
        return -1;
    }

    // Target bitrate (转换为 bps)
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_TARGET_BITRATE.as_ptr() as *const i8,
        (bitrate * 1000) as i64,
    ) != 0
    {
        return -1;
    }

    // Framerate (使用 AMFConstructRate，这里简化为 framerate << 32 | 1)
    let rate = ((framerate as i64) << 32) | 1;
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_FRAMERATE.as_ptr() as *const i8,
        rate,
    ) != 0
    {
        return -1;
    }

    // IDR period (GOP)
    let gop_value = if gop > 0 && gop < 0x7FFFFFFF {
        gop as i64
    } else {
        0x7FFFFFFF
    };
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_IDR_PERIOD.as_ptr() as *const i8,
        gop_value,
    ) != 0
    {
        return -1;
    }

    0
}

// 辅助函数：设置 HEVC 编码器参数
#[cfg(windows)]
unsafe fn set_hevc_params(
    encoder: *mut c_void,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
    enable_4k: bool,
) -> i32 {
    // Usage (可选，不检查错误)
    let _ = amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_USAGE.as_ptr() as *const i8,
        AMF_VIDEO_ENCODER_HEVC_USAGE_LOW_LATENCY,
    );

    // Frame size
    let frame_size = ((width as i64) << 32) | (height as i64);
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_FRAMESIZE.as_ptr() as *const i8,
        frame_size,
    ) != 0
    {
        return -1;
    }

    // Low latency mode
    if amf_wrapper_component_set_property_bool(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_LOWLATENCY_MODE.as_ptr() as *const i8,
        1,
    ) != 0
    {
        return -1;
    }

    // Quality preset
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET.as_ptr() as *const i8,
        AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_QUALITY,
    ) != 0
    {
        return -1;
    }

    // Color bit depth (可选)
    let _ = amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH.as_ptr() as *const i8,
        AMF_COLOR_BIT_DEPTH_8,
    );

    // Rate control method
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD.as_ptr() as *const i8,
        AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_CBR,
    ) != 0
    {
        return -1;
    }

    // 4K tier and profile level
    if enable_4k {
        if amf_wrapper_component_set_property_int64(
            encoder,
            AMF_VIDEO_ENCODER_HEVC_TIER.as_ptr() as *const i8,
            AMF_VIDEO_ENCODER_HEVC_TIER_HIGH,
        ) != 0
        {
            return -1;
        }

        if amf_wrapper_component_set_property_int64(
            encoder,
            AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL.as_ptr() as *const i8,
            AMF_LEVEL_5_1,
        ) != 0
        {
            return -1;
        }
    }

    // Nominal range (可选，默认 studio)
    let _ = amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE.as_ptr() as *const i8,
        AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE_STUDIO,
    );

    // Query timeout
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_QUERY_TIMEOUT.as_ptr() as *const i8,
        1000,
    ) != 0
    {
        return -1;
    }

    // Target bitrate
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE.as_ptr() as *const i8,
        (bitrate * 1000) as i64,
    ) != 0
    {
        return -1;
    }

    // Framerate
    let rate = ((framerate as i64) << 32) | 1;
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_FRAMERATE.as_ptr() as *const i8,
        rate,
    ) != 0
    {
        return -1;
    }

    // GOP size
    let gop_value = if gop > 0 && gop < 0x7FFFFFFF {
        gop as i64
    } else {
        0x7FFFFFFF
    };
    if amf_wrapper_component_set_property_int64(
        encoder,
        AMF_VIDEO_ENCODER_HEVC_GOP_SIZE.as_ptr() as *const i8,
        gop_value,
    ) != 0
    {
        return -1;
    }

    0
}

// 辅助函数：创建编码器
#[cfg(windows)]
pub unsafe extern "C" fn amf_new_encoder(
    handle: *mut c_void,
    luid: i64,
    data_format: i32,
    width: i32,
    height: i32,
    bitrate: i32,
    framerate: i32,
    gop: i32,
) -> *mut c_void {
    // 1. 获取编码器名称
    let codec_name = match get_codec_name(data_format) {
        Some(name) => name,
        None => return std::ptr::null_mut(),
    };

    // 2. 初始化 Factory
    let mut factory: *mut c_void = std::ptr::null_mut();
    if amf_wrapper_factory_init(&mut factory) != 0 {
        return std::ptr::null_mut();
    }

    // 3. 创建 Context
    let mut context: *mut c_void = std::ptr::null_mut();
    if amf_wrapper_create_context(factory, &mut context) != 0 {
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 4. 初始化 DX11
    if amf_wrapper_context_init_dx11(context, handle) != 0 {
        amf_wrapper_release(context);
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 5. 创建编码器组件
    let codec_name_cstr = std::ffi::CString::new(codec_name).unwrap();
    let mut encoder: *mut c_void = std::ptr::null_mut();
    if amf_wrapper_create_encoder_component(
        factory,
        context,
        codec_name_cstr.as_ptr(),
        &mut encoder,
    ) != 0
    {
        amf_wrapper_release(context);
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 6. 设置编码器参数
    let enable_4k = width > 1920 && height > 1080;
    let set_params_result = if data_format == 0 {
        // H264
        set_h264_params(encoder, width, height, bitrate, framerate, gop, enable_4k)
    } else {
        // HEVC
        set_hevc_params(encoder, width, height, bitrate, framerate, gop, enable_4k)
    };

    if set_params_result != 0 {
        amf_wrapper_component_terminate(encoder);
        amf_wrapper_release(encoder);
        amf_wrapper_release(context);
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 7. 初始化编码器
    // HEVC 需要 NV12 格式，H264 使用 BGRA
    let init_format = if data_format == 1 {
        AMF_SURFACE_NV12
    } else {
        AMF_SURFACE_BGRA
    };

    if amf_wrapper_component_init(encoder, init_format, width, height) != 0 {
        amf_wrapper_component_terminate(encoder);
        amf_wrapper_release(encoder);
        amf_wrapper_release(context);
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 8. 创建转换器（仅 HEVC 需要，用于 BGRA -> NV12）
    let mut converter: *mut c_void = std::ptr::null_mut();
    if data_format == 1 {
        // HEVC 需要转换器
        if amf_wrapper_create_converter_component(factory, context, &mut converter) != 0 {
            amf_wrapper_component_terminate(encoder);
            amf_wrapper_release(encoder);
            amf_wrapper_release(context);
            amf_wrapper_factory_terminate(factory);
            return std::ptr::null_mut();
        }

        // 设置转换器参数
        if amf_wrapper_component_set_property_int64(
            converter,
            AMF_VIDEO_CONVERTER_MEMORY_TYPE.as_ptr() as *const i8,
            AMF_MEMORY_DX11 as i64,
        ) != 0
        {
            amf_wrapper_release(converter);
            amf_wrapper_component_terminate(encoder);
            amf_wrapper_release(encoder);
            amf_wrapper_release(context);
            amf_wrapper_factory_terminate(factory);
            return std::ptr::null_mut();
        }

        if amf_wrapper_component_set_property_int64(
            converter,
            AMF_VIDEO_CONVERTER_OUTPUT_FORMAT.as_ptr() as *const i8,
            AMF_SURFACE_NV12 as i64,
        ) != 0
        {
            amf_wrapper_release(converter);
            amf_wrapper_component_terminate(encoder);
            amf_wrapper_release(encoder);
            amf_wrapper_release(context);
            amf_wrapper_factory_terminate(factory);
            return std::ptr::null_mut();
        }

        let output_size = ((width as i64) << 32) | (height as i64);
        if amf_wrapper_component_set_property_int64(
            converter,
            AMF_VIDEO_CONVERTER_OUTPUT_SIZE.as_ptr() as *const i8,
            output_size,
        ) != 0
        {
            amf_wrapper_release(converter);
            amf_wrapper_component_terminate(encoder);
            amf_wrapper_release(encoder);
            amf_wrapper_release(context);
            amf_wrapper_factory_terminate(factory);
            return std::ptr::null_mut();
        }

        // 初始化转换器
        if amf_wrapper_component_init(converter, AMF_SURFACE_BGRA, width, height) != 0 {
            amf_wrapper_release(converter);
            amf_wrapper_component_terminate(encoder);
            amf_wrapper_release(encoder);
            amf_wrapper_release(context);
            amf_wrapper_factory_terminate(factory);
            return std::ptr::null_mut();
        }
    }

    // 9. 创建编码器结构
    let data_format_enum = match data_format {
        0 => H264,
        1 => H265,
        _ => return std::ptr::null_mut(),
    };

    let encoder_struct = Box::new(AmfEncoder {
        factory,
        context,
        encoder,
        converter,
        device: handle,
        data_format: data_format_enum,
        width,
        height,
        bitrate: bitrate * 1000, // 转换为 bps
        framerate,
        gop: if gop > 0 && gop < 0x7FFFFFFF {
            gop
        } else {
            0x7FFFFFFF
        },
        packet_buffer: Vec::new(),
    });

    Box::into_raw(encoder_struct) as *mut c_void
}

// 辅助函数：检查是否为关键帧
unsafe fn is_keyframe(data: *mut c_void, data_format: i32) -> bool {
    // 注意：这需要从 AMF 数据中获取属性，但 C 包装层没有提供 GetProperty 函数
    // 暂时返回 false，后续可以通过扩展 C 包装层来支持
    // 或者通过检查数据包类型来判断
    false
}

// 辅助函数：编码
#[cfg(windows)]
pub unsafe extern "C" fn amf_encode(
    encoder: *mut c_void,
    texture: *mut c_void,
    callback: EncodeCallback,
    obj: *mut c_void,
    ms: i64,
) -> i32 {
    if encoder.is_null() || texture.is_null() {
        return -1;
    }

    let enc = &*(encoder as *const AmfEncoder);

    // 1. 从 D3D11 纹理创建 Surface
    let mut surface: *mut c_void = std::ptr::null_mut();
    if amf_wrapper_create_surface_from_dx11(enc.context, texture, &mut surface) != 0 {
        return -1;
    }

    // 2. 复制 Surface（AMF 要求）
    let mut duplicated_surface: *mut c_void = std::ptr::null_mut();
    if amf_wrapper_surface_duplicate(surface, AMF_MEMORY_DX11, &mut duplicated_surface) != 0 {
        amf_wrapper_release(surface);
        return -1;
    }
    amf_wrapper_release(surface);
    surface = duplicated_surface;

    // 3. 设置 PTS (AMF_MILLISECOND = 10000)
    let pts = ms * 10000;
    amf_wrapper_surface_set_pts(surface, pts);

    // 4. 如果是 HEVC，使用转换器将 BGRA 转换为 NV12
    let mut input_surface = surface;
    if enc.data_format == H265 && !enc.converter.is_null() {
        // 提交到转换器
        if amf_wrapper_converter_submit_input(enc.converter, surface) != 0 {
            amf_wrapper_release(surface);
            return -1;
        }

        // 查询转换器输出
        let mut converted_data: *mut c_void = std::ptr::null_mut();
        let mut query_result = amf_wrapper_converter_query_output(enc.converter, &mut converted_data);
        
        // 释放原始 surface
        amf_wrapper_release(surface);
        
        if query_result != 0 {
            return -1;
        }
        
        input_surface = converted_data;
    }

    // 5. 提交输入到编码器
    if amf_wrapper_encoder_submit_input(enc.encoder, input_surface) != 0 {
        if enc.data_format == H265 {
            amf_wrapper_release(input_surface);
        }
        return -1;
    }

    // 如果使用了转换器，释放转换后的 surface
    if enc.data_format == H265 {
        amf_wrapper_release(input_surface);
    }

    // 6. 查询编码器输出
    let mut output_data: *mut c_void = std::ptr::null_mut();
    let query_result = amf_wrapper_encoder_query_output(enc.encoder, &mut output_data);
    
    if query_result != 0 {
        // 0=成功, -1=失败, 1=需要更多输入
        return if query_result == 1 { 1 } else { -1 };
    }

    if output_data.is_null() {
        return -1;
    }

    // 7. 获取输出数据包
    let buffer_size = amf_wrapper_buffer_get_size(output_data);
    if buffer_size <= 0 {
        amf_wrapper_release(output_data);
        return -1;
    }

    let buffer_ptr = amf_wrapper_buffer_get_native(output_data);
    if buffer_ptr.is_null() {
        amf_wrapper_release(output_data);
        return -1;
    }

    // 8. 复制数据到缓冲区（如果需要扩展缓冲区）
    let enc_mut = &mut *(encoder as *mut AmfEncoder);
    let required_size = buffer_size as usize;
    if enc_mut.packet_buffer.len() < required_size {
        // 扩展到下一个 2 的幂
        let new_size = (required_size as f64).log2().ceil().exp2() as usize;
        enc_mut.packet_buffer.resize(new_size, 0);
    }

    // 9. 复制数据
    std::ptr::copy_nonoverlapping(
        buffer_ptr as *const u8,
        enc_mut.packet_buffer.as_mut_ptr(),
        required_size,
    );

    // 10. 检查是否为关键帧
    let is_keyframe = if enc.data_format == H264 {
        // H264: 检查 AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE
        let mut pkt_type: i64 = 0;
        let prop_name = b"AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE\0";
        if amf_wrapper_buffer_get_property_int64(
            output_data,
            prop_name.as_ptr() as *const c_char,
            &mut pkt_type,
        ) == 0 {
            // AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_IDR = 1
            // AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_I = 2
            pkt_type == 1 || pkt_type == 2
        } else {
            false
        }
    } else if enc.data_format == H265 {
        // H265: 检查 AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE
        let mut pkt_type: i64 = 0;
        let prop_name = b"AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE\0";
        if amf_wrapper_buffer_get_property_int64(
            output_data,
            prop_name.as_ptr() as *const c_char,
            &mut pkt_type,
        ) == 0 {
            // AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_IDR = 1
            // AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_I = 2
            pkt_type == 1 || pkt_type == 2
        } else {
            false
        }
    } else {
        false
    };

    // 11. 调用回调函数
    if let Some(cb) = callback {
        cb(
            enc_mut.packet_buffer.as_ptr(),
            required_size as i32,
            if is_keyframe { 1 } else { 0 },
            obj,
            ms,
        );
    }

    // 12. 释放输出数据
    amf_wrapper_release(output_data);

    0
}

// 辅助函数：销毁编码器
#[cfg(windows)]
pub unsafe extern "C" fn amf_destroy_encoder(encoder: *mut c_void) -> i32 {
    if encoder.is_null() {
        return 0;
    }

    let enc = Box::from_raw(encoder as *mut AmfEncoder);

    // 1. 终止转换器
    if !enc.converter.is_null() {
        amf_wrapper_component_terminate(enc.converter);
        amf_wrapper_release(enc.converter);
    }

    // 2. 终止编码器
    if !enc.encoder.is_null() {
        amf_wrapper_component_terminate(enc.encoder);
        amf_wrapper_release(enc.encoder);
    }

    // 3. 终止 Context
    if !enc.context.is_null() {
        amf_wrapper_release(enc.context);
    }

    // 4. 终止 Factory
    if !enc.factory.is_null() {
        amf_wrapper_factory_terminate(enc.factory);
    }

    0
}

// 解码器常量
const AMFVideoDecoderUVD_H264_AVC: &str = "AMFVideoDecoderUVD_H264_AVC";
const AMFVideoDecoderHW_H265_HEVC: &str = "AMFVideoDecoderHW_H265_HEVC";

// 解码器属性
const AMF_TIMESTAMP_MODE: &str = "AMF_TIMESTAMP_MODE";
const AMF_TS_DECODE: i64 = 1;
const AMF_VIDEO_DECODER_REORDER_MODE: &str = "AMF_VIDEO_DECODER_REORDER_MODE";
const AMF_VIDEO_DECODER_MODE_LOW_LATENCY: i64 = 0;
const AMF_VIDEO_DECODER_COLOR_RANGE: &str = "AMF_VIDEO_DECODER_COLOR_RANGE";
const AMF_COLOR_RANGE_FULL: i64 = 0;
const AMF_COLOR_RANGE_STUDIO: i64 = 1;
const AMF_VIDEO_DECODER_COLOR_PROFILE: &str = "AMF_VIDEO_DECODER_COLOR_PROFILE";
const AMF_VIDEO_CONVERTER_COLOR_PROFILE_709: i64 = 1;
const AMF_VIDEO_CONVERTER_COLOR_PROFILE_FULL_709: i64 = 2;
const AMF_VIDEO_CONVERTER_COLOR_PROFILE_601: i64 = 0;
const AMF_VIDEO_CONVERTER_COLOR_PROFILE_FULL_601: i64 = 3;
const AMF_COLOR_TRANSFER_CHARACTERISTIC_BT709: i64 = 1;
const AMF_COLOR_TRANSFER_CHARACTERISTIC_SMPTE170M: i64 = 4;
const AMF_COLOR_PRIMARIES_BT709: i64 = 1;
const AMF_COLOR_PRIMARIES_SMPTE170M: i64 = 6;

// 辅助函数：将 DataFormat 转换为解码器名称
fn get_decoder_codec_name(data_format: i32) -> Option<&'static str> {
    match data_format {
        0 => Some(AMFVideoDecoderUVD_H264_AVC), // H264
        1 => Some(AMFVideoDecoderHW_H265_HEVC), // H265
        _ => None,
    }
}

// 辅助函数：设置解码器参数
#[cfg(windows)]
unsafe fn set_decoder_params(decoder: *mut c_void) -> i32 {
    // Timestamp mode
    if amf_wrapper_component_set_property_int64(
        decoder,
        AMF_TIMESTAMP_MODE.as_ptr() as *const i8,
        AMF_TS_DECODE,
    ) != 0
    {
        return -1;
    }

    // Reorder mode (low latency)
    if amf_wrapper_component_set_property_int64(
        decoder,
        AMF_VIDEO_DECODER_REORDER_MODE.as_ptr() as *const i8,
        AMF_VIDEO_DECODER_MODE_LOW_LATENCY,
    ) != 0
    {
        return -1;
    }

    // Color range (可选，默认 studio)
    let _ = amf_wrapper_component_set_property_int64(
        decoder,
        AMF_VIDEO_DECODER_COLOR_RANGE.as_ptr() as *const i8,
        AMF_COLOR_RANGE_STUDIO,
    );

    // Color profile (可选)
    let _ = amf_wrapper_component_set_property_int64(
        decoder,
        AMF_VIDEO_DECODER_COLOR_PROFILE.as_ptr() as *const i8,
        AMF_VIDEO_CONVERTER_COLOR_PROFILE_601,
    );

    0
}

// 辅助函数：创建解码器
#[cfg(windows)]
pub unsafe extern "C" fn amf_new_decoder(
    device: *mut c_void,
    luid: i64,
    data_format: i32,
) -> *mut c_void {
    // 1. 获取解码器名称
    let codec_name = match get_decoder_codec_name(data_format) {
        Some(name) => name,
        None => return std::ptr::null_mut(),
    };

    // 2. 初始化 Factory
    let mut factory: *mut c_void = std::ptr::null_mut();
    if amf_wrapper_factory_init(&mut factory) != 0 {
        return std::ptr::null_mut();
    }

    // 3. 创建 Context
    let mut context: *mut c_void = std::ptr::null_mut();
    if amf_wrapper_create_context(factory, &mut context) != 0 {
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 4. 创建 NativeDevice（用于纹理管理）
    // 声明 NativeDevice FFI 函数（在函数外部声明，避免重复）
    extern "C" {
        fn hwcodec_native_device_new(
            luid: i64,
            device: *mut c_void,
            pool_size: i32,
        ) -> *mut c_void;
        fn hwcodec_native_device_get_device(handle: *mut c_void) -> *mut c_void;
        fn hwcodec_native_device_destroy(handle: *mut c_void);
        fn hwcodec_native_device_ensure_texture(handle: *mut c_void, width: u32, height: u32) -> i32;
        fn hwcodec_native_device_next(handle: *mut c_void) -> i32;
        fn hwcodec_native_device_get_current_texture(handle: *mut c_void) -> *mut c_void;
        fn hwcodec_native_device_get_context(handle: *mut c_void) -> *mut c_void;
    }
    
    let native_device = hwcodec_native_device_new(luid, device, 4);
    if native_device.is_null() {
        amf_wrapper_release(context);
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 5. 从 NativeDevice 获取设备
    let amf_device = hwcodec_native_device_get_device(native_device);
    if amf_device.is_null() {
        hwcodec_native_device_destroy(native_device);
        amf_wrapper_release(context);
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 6. 初始化 DX11
    if amf_wrapper_context_init_dx11(context, amf_device) != 0 {
        hwcodec_native_device_destroy(native_device);
        amf_wrapper_release(context);
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 7. 创建解码器组件
    let codec_name_cstr = std::ffi::CString::new(codec_name).unwrap();
    let mut decoder: *mut c_void = std::ptr::null_mut();
    if amf_wrapper_create_decoder_component(
        factory,
        context,
        codec_name_cstr.as_ptr(),
        &mut decoder,
    ) != 0
    {
        hwcodec_native_device_destroy(native_device);
        amf_wrapper_release(context);
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 8. 设置解码器参数
    if set_decoder_params(decoder) != 0 {
        amf_wrapper_component_terminate(decoder);
        amf_wrapper_release(decoder);
        hwcodec_native_device_destroy(native_device);
        amf_wrapper_release(context);
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 9. 初始化解码器（格式 NV12，宽度和高度为 0，由输入流决定）
    if amf_wrapper_component_init(decoder, AMF_SURFACE_NV12, 0, 0) != 0 {
        amf_wrapper_component_terminate(decoder);
        amf_wrapper_release(decoder);
        hwcodec_native_device_destroy(native_device);
        amf_wrapper_release(context);
        amf_wrapper_factory_terminate(factory);
        return std::ptr::null_mut();
    }

    // 10. 创建转换器（NV12 -> BGRA，但延迟初始化，因为尺寸未知）
    // 转换器将在第一次解码时根据实际尺寸创建
    let converter: *mut c_void = std::ptr::null_mut();

    // 11. 创建解码器结构
    let data_format_enum = match data_format {
        0 => H264,
        1 => H265,
        _ => {
            amf_wrapper_release(converter);
            amf_wrapper_component_terminate(decoder);
            amf_wrapper_release(decoder);
            hwcodec_native_device_destroy(native_device);
            amf_wrapper_release(context);
            amf_wrapper_factory_terminate(factory);
            return std::ptr::null_mut();
        }
    };

    let decoder_struct = Box::new(AmfDecoder {
        factory,
        context,
        decoder,
        converter,
        device: amf_device,
        native_device,
        luid,
        data_format: data_format_enum,
        last_width: 0,
        last_height: 0,
    });

    Box::into_raw(decoder_struct) as *mut c_void
}

// 辅助函数：解码
#[cfg(windows)]
pub unsafe extern "C" fn amf_decode(
    decoder: *mut c_void,
    data: *mut u8,
    length: i32,
    callback: DecodeCallback,
    obj: *mut c_void,
) -> i32 {
    if decoder.is_null() || data.is_null() || length <= 0 {
        return -1;
    }

    let dec = &mut *(decoder as *mut AmfDecoder);

    // 1. 提交输入数据（C 包装层会创建缓冲区并复制数据）
    // PTS 设为 0（由 AMF 自动处理）
    let mut submit_result = amf_wrapper_decoder_submit_input(dec.decoder, data, length, 0);
    
    // 处理分辨率变化
    if submit_result == AMF_RESOLUTION_CHANGED {
        // 分辨率变化，需要重新初始化解码器
        // 1. Drain 解码器
        if amf_wrapper_component_drain(dec.decoder) != 0 {
            return -1;
        }
        
        // 2. Terminate 解码器
        amf_wrapper_component_terminate(dec.decoder);
        
        // 3. 重新初始化解码器（格式 NV12，宽度和高度为 0，由输入流决定）
        if amf_wrapper_component_init(dec.decoder, AMF_SURFACE_NV12, 0, 0) != 0 {
            return -1;
        }
        
        // 4. 重置转换器状态（尺寸会变化）
        if !dec.converter.is_null() {
            amf_wrapper_component_terminate(dec.converter);
            amf_wrapper_release(dec.converter);
            dec.converter = std::ptr::null_mut();
            dec.last_width = 0;
            dec.last_height = 0;
        }
        
        // 5. 重新提交输入（C 包装层会重新创建 buffer）
        submit_result = amf_wrapper_decoder_submit_input(dec.decoder, data, length, 0);
        
        if submit_result != 0 {
            return -1;
        }
    } else if submit_result != 0 {
        return -1;
    }

    // 2. 查询输出 Surface（循环查询直到成功或超时）
    let mut output_surface: *mut c_void = std::ptr::null_mut();
    let start_time = std::time::Instant::now();
    let mut query_result = amf_wrapper_decoder_query_output(dec.decoder, &mut output_surface);
    
    // DECODE_TIMEOUT_MS 在 common_ffi.rs 中定义为 u32，转换为 u128
    // 直接使用常量值 1000ms
    const TIMEOUT_MS: u128 = 1000;
    while query_result == 1 && start_time.elapsed().as_millis() < TIMEOUT_MS {
        // AMF_REPEAT (1) = 需要更多输入，等待
        std::thread::sleep(std::time::Duration::from_millis(1));
        query_result = amf_wrapper_decoder_query_output(dec.decoder, &mut output_surface);
    }

    if query_result != 0 || output_surface.is_null() {
        return -1;
    }

    // 3. 检查 Surface 平面数
    let planes_count = amf_wrapper_surface_get_planes_count(output_surface);
    if planes_count == 0 {
        amf_wrapper_release(output_surface);
        return -1;
    }

    // 4. 使用转换器转换格式（NV12 -> BGRA）
    // 获取当前 Surface 尺寸
    let width = amf_wrapper_surface_get_width(output_surface);
    let height = amf_wrapper_surface_get_height(output_surface);

    // 动态创建或重新初始化转换器（如果尺寸变化）
    let mut converter = dec.converter;
    if converter.is_null() || width != dec.last_width || height != dec.last_height {
        // 如果已有转换器但尺寸变化，先终止并释放
        if !converter.is_null() {
            amf_wrapper_component_terminate(converter);
            amf_wrapper_release(converter);
        }

        // 创建新的转换器
        let mut new_converter: *mut c_void = std::ptr::null_mut();
        if amf_wrapper_create_converter_component(dec.factory, dec.context, &mut new_converter) != 0 {
            amf_wrapper_release(output_surface);
            return -1;
        }

        // 设置转换器属性
        // MEMORY_TYPE = DX11
        if amf_wrapper_component_set_property_int64(
            new_converter,
            b"AMF_VIDEO_CONVERTER_MEMORY_TYPE\0".as_ptr() as *const i8,
            AMF_MEMORY_DX11 as i64,
        ) != 0 {
            amf_wrapper_release(new_converter);
            amf_wrapper_release(output_surface);
            return -1;
        }

        // OUTPUT_FORMAT = BGRA
        if amf_wrapper_component_set_property_int64(
            new_converter,
            b"AMF_VIDEO_CONVERTER_OUTPUT_FORMAT\0".as_ptr() as *const i8,
            AMF_SURFACE_BGRA as i64,
        ) != 0 {
            amf_wrapper_release(new_converter);
            amf_wrapper_release(output_surface);
            return -1;
        }

        // OUTPUT_SIZE = width x height
        // AMFConstructSize: (width << 32) | height
        let size = ((width as i64) << 32) | (height as i64);
        if amf_wrapper_component_set_property_int64(
            new_converter,
            b"AMF_VIDEO_CONVERTER_OUTPUT_SIZE\0".as_ptr() as *const i8,
            size,
        ) != 0 {
            amf_wrapper_release(new_converter);
            amf_wrapper_release(output_surface);
            return -1;
        }

        // 初始化转换器（输入格式 NV12，输出格式 BGRA）
        if amf_wrapper_component_init(new_converter, AMF_SURFACE_NV12, width, height) != 0 {
            amf_wrapper_release(new_converter);
            amf_wrapper_release(output_surface);
            return -1;
        }

        // 设置颜色空间属性（可选，不检查错误）
        // INPUT_COLOR_RANGE（默认 studio）
        let _ = amf_wrapper_component_set_property_int64(
            new_converter,
            b"AMF_VIDEO_CONVERTER_INPUT_COLOR_RANGE\0".as_ptr() as *const i8,
            AMF_COLOR_RANGE_STUDIO,
        );
        
        // OUTPUT_COLOR_RANGE（默认 full）
        let _ = amf_wrapper_component_set_property_int64(
            new_converter,
            b"AMF_VIDEO_CONVERTER_OUTPUT_COLOR_RANGE\0".as_ptr() as *const i8,
            AMF_COLOR_RANGE_FULL,
        );
        
        // COLOR_PROFILE（默认 601）
        let _ = amf_wrapper_component_set_property_int64(
            new_converter,
            b"AMF_VIDEO_CONVERTER_COLOR_PROFILE\0".as_ptr() as *const i8,
            AMF_VIDEO_CONVERTER_COLOR_PROFILE_601,
        );
        
        // INPUT_TRANSFER_CHARACTERISTIC（默认 SMPTE170M）
        let _ = amf_wrapper_component_set_property_int64(
            new_converter,
            b"AMF_VIDEO_CONVERTER_INPUT_TRANSFER_CHARACTERISTIC\0".as_ptr() as *const i8,
            AMF_COLOR_TRANSFER_CHARACTERISTIC_SMPTE170M,
        );
        
        // INPUT_COLOR_PRIMARIES（默认 SMPTE170M）
        let _ = amf_wrapper_component_set_property_int64(
            new_converter,
            b"AMF_VIDEO_CONVERTER_INPUT_COLOR_PRIMARIES\0".as_ptr() as *const i8,
            AMF_COLOR_PRIMARIES_SMPTE170M,
        );

        converter = new_converter;
        // 更新解码器结构中的转换器指针
        dec.converter = converter;
        dec.last_width = width;
        dec.last_height = height;
    }

    // 提交到转换器
    if amf_wrapper_converter_submit_input(converter, output_surface) != 0 {
        amf_wrapper_release(output_surface);
        return -1;
    }

    // 查询转换器输出
    let mut converted_data: *mut c_void = std::ptr::null_mut();
    let convert_query_result = amf_wrapper_converter_query_output(converter, &mut converted_data);
    
    // 释放原始 surface
    amf_wrapper_release(output_surface);
    
    if convert_query_result != 0 {
        return -1;
    }
    
    let converted_surface = converted_data;

    // 5. 获取转换后的 Surface 平面
    let converted_planes_count = amf_wrapper_surface_get_planes_count(converted_surface);
    if converted_planes_count == 0 {
        amf_wrapper_release(converted_surface);
        return -1;
    }

    // 6. 获取第一个平面的原生纹理指针
    let plane = amf_wrapper_surface_get_plane_at(converted_surface, 0);
    if plane.is_null() {
        amf_wrapper_release(converted_surface);
        return -1;
    }

    let native_texture = amf_wrapper_plane_get_native(plane);
    if native_texture.is_null() {
        amf_wrapper_release(converted_surface);
        return -1;
    }

    // 7. 获取 Surface 尺寸并确保 NativeDevice 纹理
    let width_u32 = amf_wrapper_surface_get_width(converted_surface) as u32;
    let height_u32 = amf_wrapper_surface_get_height(converted_surface) as u32;

    extern "C" {
        fn hwcodec_native_device_ensure_texture(handle: *mut c_void, width: u32, height: u32) -> i32;
        fn hwcodec_native_device_next(handle: *mut c_void) -> i32;
        fn hwcodec_native_device_get_current_texture(handle: *mut c_void) -> *mut c_void;
        fn hwcodec_native_device_get_context(handle: *mut c_void) -> *mut c_void;
    }

    if hwcodec_native_device_ensure_texture(dec.native_device, width_u32, height_u32) != 0 {
        amf_wrapper_release(converted_surface);
        return -1;
    }

    // 8. 获取下一个纹理并复制
    if hwcodec_native_device_next(dec.native_device) != 0 {
        amf_wrapper_release(converted_surface);
        return -1;
    }

    let dst_texture = hwcodec_native_device_get_current_texture(dec.native_device);
    if dst_texture.is_null() {
        amf_wrapper_release(converted_surface);
        return -1;
    }

    // 9. 复制纹理（使用 D3D11 Context）
    let context = hwcodec_native_device_get_context(dec.native_device);
    if context.is_null() {
        if dec.converter.is_null() {
            amf_wrapper_release(converted_surface);
        } else {
            amf_wrapper_release(converted_surface);
        }
        return -1;
    }

    // 使用 Windows API 复制资源
    // 通过 windows-rs crate 调用 D3D11 API
    use windows::Win32::Graphics::Direct3D11::{ID3D11DeviceContext, ID3D11Texture2D};
    use windows::core::Interface;
    
    unsafe {
        // 从原始指针创建接口（不增加引用计数，因为这是外部管理的）
        let src_texture: ID3D11Texture2D = Interface::from_raw(native_texture as *mut std::ffi::c_void);
        let dst_texture_ptr: ID3D11Texture2D = Interface::from_raw(dst_texture as *mut std::ffi::c_void);
        let context_ptr: ID3D11DeviceContext = Interface::from_raw(context as *mut std::ffi::c_void);
        
        let _ = context_ptr.CopyResource(&dst_texture_ptr, &src_texture);
        context_ptr.Flush();
        
        // 不要释放，因为这是外部管理的指针
        std::mem::forget(src_texture);
        std::mem::forget(dst_texture_ptr);
        std::mem::forget(context_ptr);
    }

    // 10. 调用回调函数
    if let Some(cb) = callback {
        cb(dst_texture, obj);
    }

    // 11. 释放资源
    amf_wrapper_release(converted_surface);

    0
}

// 辅助函数：销毁解码器
#[cfg(windows)]
pub unsafe extern "C" fn amf_destroy_decoder(decoder: *mut c_void) -> i32 {
    if decoder.is_null() {
        return 0;
    }

    let dec = Box::from_raw(decoder as *mut AmfDecoder);

    extern "C" {
        fn hwcodec_native_device_destroy(handle: *mut c_void);
    }

    // 1. 终止转换器（先 Drain 再 Terminate）
    if !dec.converter.is_null() {
        amf_wrapper_component_drain(dec.converter);
        amf_wrapper_component_terminate(dec.converter);
        amf_wrapper_release(dec.converter);
    }

    // 2. 终止解码器（先 Drain 再 Terminate）
    if !dec.decoder.is_null() {
        amf_wrapper_component_drain(dec.decoder);
        amf_wrapper_component_terminate(dec.decoder);
        amf_wrapper_release(dec.decoder);
    }

    // 3. 释放 NativeDevice
    if !dec.native_device.is_null() {
        hwcodec_native_device_destroy(dec.native_device);
    }

    // 4. 终止 Context
    if !dec.context.is_null() {
        amf_wrapper_release(dec.context);
    }

    // 5. 终止 Factory
    if !dec.factory.is_null() {
        amf_wrapper_factory_terminate(dec.factory);
    }

    0
}

// 辅助函数：测试编码
#[cfg(windows)]
pub unsafe extern "C" fn amf_test_encode(
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
        fn hwcodec_native_device_new(luid: i64, device: *mut c_void, pool_size: i32) -> *mut c_void;
        fn hwcodec_native_device_destroy(handle: *mut c_void);
        fn hwcodec_native_device_ensure_texture(handle: *mut c_void, width: u32, height: u32) -> i32;
        fn hwcodec_native_device_next(handle: *mut c_void) -> i32;
        fn hwcodec_native_device_get_current_texture(handle: *mut c_void) -> *mut c_void;
    }

    // ADAPTER_VENDOR_AMD = 0x1002
    let adapters = hwcodec_adapters_new(0x1002);
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
        let encoder = amf_new_encoder(device, current_luid, data_format, width, height, kbs, framerate, gop);
        if encoder.is_null() {
            continue;
        }

        // 测试编码：创建临时的 native_device 用于获取测试纹理
        let mut key_obj = 0i32;
        
        let test_native_device = hwcodec_native_device_new(current_luid, device, 4);
        if !test_native_device.is_null() {
            if hwcodec_native_device_ensure_texture(test_native_device, width as u32, height as u32) == 0 {
                if hwcodec_native_device_next(test_native_device) == 0 {
                    let current_texture = hwcodec_native_device_get_current_texture(test_native_device);
                    if !current_texture.is_null() {
                        let start_time = std::time::Instant::now();
                        let succ = amf_encode(encoder, current_texture, Some(test_encode_callback), &mut key_obj as *mut i32 as *mut c_void, 0) == 0 && key_obj == 1;
                        let elapsed = start_time.elapsed().as_millis();
                        if succ && elapsed < 1000 {
                            if !out_luids.is_null() {
                                *out_luids.offset(count as isize) = current_luid;
                            }
                            if !out_vendors.is_null() {
                                *out_vendors.offset(count as isize) = 1; // VENDOR_AMD
                            }
                            count += 1;
                        }
                    }
                }
            }
            hwcodec_native_device_destroy(test_native_device);
        }

        amf_destroy_encoder(encoder);
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
pub unsafe extern "C" fn amf_test_encode(
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
pub unsafe extern "C" fn amf_test_decode(
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

    // ADAPTER_VENDOR_AMD = 0x1002
    let adapters = hwcodec_adapters_new(0x1002);
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
        let decoder = amf_new_decoder(std::ptr::null_mut(), current_luid, data_format);
        if decoder.is_null() {
            continue;
        }

        // 测试解码
        let start_time = std::time::Instant::now();
        let succ = amf_decode(decoder, data, length, None, std::ptr::null_mut()) == 0;
        let elapsed = start_time.elapsed().as_millis();
        if succ && elapsed < 1000 {
            if !out_luids.is_null() {
                *out_luids.offset(count as isize) = current_luid;
            }
            if !out_vendors.is_null() {
                *out_vendors.offset(count as isize) = 1; // VENDOR_AMD
            }
            count += 1;
        }

        amf_destroy_decoder(decoder);
    }

    hwcodec_adapters_destroy(adapters);

    if !out_desc_num.is_null() {
        *out_desc_num = count;
    }

    0
}

#[cfg(not(windows))]
pub unsafe extern "C" fn amf_test_decode(
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
pub unsafe extern "C" fn amf_set_bitrate(encoder: *mut c_void, kbs: i32) -> i32 {
    if encoder.is_null() {
        return -1;
    }

    let enc = &*(encoder as *const AmfEncoder);
    let bitrate_bps = (kbs * 1000) as i64;

    let result = match enc.data_format {
        H264 => amf_wrapper_component_set_property_int64(
            enc.encoder,
            AMF_VIDEO_ENCODER_TARGET_BITRATE.as_ptr() as *const i8,
            bitrate_bps,
        ),
        H265 => amf_wrapper_component_set_property_int64(
            enc.encoder,
            AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE.as_ptr() as *const i8,
            bitrate_bps,
        ),
        _ => -1,
    };

    if result == 0 {
        // 更新内部状态
        let enc_mut = &mut *(encoder as *mut AmfEncoder);
        enc_mut.bitrate = kbs * 1000;
        0
    } else {
        -1
    }
}

// 辅助函数：设置帧率
#[cfg(windows)]
pub unsafe extern "C" fn amf_set_framerate(encoder: *mut c_void, framerate: i32) -> i32 {
    if encoder.is_null() {
        return -1;
    }

    let enc = &*(encoder as *const AmfEncoder);
    // AMFConstructRate: framerate << 32 | 1
    let rate = ((framerate as i64) << 32) | 1;

    let result = match enc.data_format {
        H264 => amf_wrapper_component_set_property_int64(
            enc.encoder,
            AMF_VIDEO_ENCODER_FRAMERATE.as_ptr() as *const i8,
            rate,
        ),
        H265 => amf_wrapper_component_set_property_int64(
            enc.encoder,
            AMF_VIDEO_ENCODER_HEVC_FRAMERATE.as_ptr() as *const i8,
            rate,
        ),
        _ => -1,
    };

    if result == 0 {
        // 更新内部状态
        let enc_mut = &mut *(encoder as *mut AmfEncoder);
        enc_mut.framerate = framerate;
        0
    } else {
        -1
    }
}

pub fn encode_calls() -> EncodeCalls {
    EncodeCalls {
        new: amf_new_encoder,
        encode: amf_encode,
        destroy: amf_destroy_encoder,
        test: amf_test_encode,
        set_bitrate: amf_set_bitrate,
        set_framerate: amf_set_framerate,
    }
}

pub fn decode_calls() -> DecodeCalls {
    DecodeCalls {
        new: amf_new_decoder,
        decode: amf_decode,
        destroy: amf_destroy_decoder,
        test: amf_test_decode,
    }
}

pub fn possible_support_encoders() -> Vec<InnerEncodeContext> {
    if amf_driver_support() != 0 {
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
    if amf_driver_support() != 0 {
        return vec![];
    }
    let data_formats = vec![H264];
    let mut v = vec![];
    for data_format in data_formats.iter() {
        v.push(InnerDecodeContext {
            data_format: data_format.clone(),
        });
    }
    v
}
