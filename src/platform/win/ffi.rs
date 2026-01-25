//! FFI 接口，用于与 C++ 代码兼容
//! 
//! 提供 C 风格的函数接口，替换 C++ 实现。
//! 
//! 这些函数替换了 `cpp/common/platform/win/win.cpp` 中的以下函数：
//! - `GetHwcodecGpuSignature()` - 替换 C++ 实现（已注释）
//! - `hwcodec_get_d3d11_texture_width_height()` - 替换 C++ 实现（已注释）
//! - `add_process_to_new_job()` - 替换 C++ 实现（已注释）
//! - `NativeDevice`, `Adapter`, `Adapters` 类 - 通过不透明指针提供 FFI 接口

use crate::common;
use crate::platform::win::adapter::Adapters;
use crate::platform::win::bmp;
use crate::platform::win::device::NativeDevice;
use crate::platform::win::dump;
use crate::platform::win::texture;
use crate::platform::win::utils;
use std::ffi::c_int;
use std::ptr;
use windows::core::Interface;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;

/// 获取 GPU 签名（用于检测 GPU 驱动更新）
#[no_mangle]
pub extern "C" fn GetHwcodecGpuSignature() -> u64 {
    utils::get_gpu_signature()
}

/// 获取 D3D11 纹理的宽度和高度
#[no_mangle]
pub extern "C" fn hwcodec_get_d3d11_texture_width_height(
    texture: *mut std::ffi::c_void,
    width: *mut i32,
    height: *mut i32,
) {
    if texture.is_null() || width.is_null() || height.is_null() {
        return;
    }

    unsafe {
        let texture_ptr = texture as *mut ID3D11Texture2D;
        // 从原始指针创建接口（不增加引用计数，因为这是外部管理的）
        let texture = Interface::from_raw(texture_ptr as *mut std::ffi::c_void);
        
        match texture::get_texture_width_height(&texture) {
            Ok((w, h)) => {
                *width = w as i32;
                *height = h as i32;
            }
            Err(_) => {
                *width = 0;
                *height = 0;
            }
        }
        
        // 不要释放，因为这是外部管理的指针
        std::mem::forget(texture);
    }
}

/// 将进程添加到新的作业对象
/// 当作业对象关闭时，子进程会自动终止
#[no_mangle]
pub extern "C" fn add_process_to_new_job(process_id: u32) -> i32 {
    match utils::add_process_to_new_job(process_id) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// ============================================================================
// NativeDevice FFI 接口
// ============================================================================

/// 不透明的 NativeDevice 指针类型
pub type NativeDeviceHandle = *mut NativeDevice;

/// 创建新的 NativeDevice
/// 
/// # 参数
/// - `luid`: 适配器 LUID，如果为 0 则使用默认适配器
/// - `device`: 可选的现有 D3D11 设备指针，如果为 nullptr 则从 LUID 创建
/// - `pool_size`: 纹理池大小
/// 
/// # 返回
/// - 成功：返回 NativeDevice 句柄
/// - 失败：返回 nullptr
#[no_mangle]
pub extern "C" fn hwcodec_native_device_new(
    luid: i64,
    device: *mut std::ffi::c_void,
    pool_size: c_int,
) -> NativeDeviceHandle {
    let device_ptr = if device.is_null() {
        None
    } else {
        Some(device as *mut ID3D11Device)
    };

    match NativeDevice::new(luid, device_ptr, pool_size as usize) {
        Ok(native_device) => Box::into_raw(Box::new(native_device)),
        Err(_) => ptr::null_mut(),
    }
}

/// 释放 NativeDevice
#[no_mangle]
pub extern "C" fn hwcodec_native_device_destroy(handle: NativeDeviceHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// 初始化纹理池
#[no_mangle]
pub extern "C" fn hwcodec_native_device_ensure_texture(
    handle: NativeDeviceHandle,
    width: u32,
    height: u32,
) -> c_int {
    if handle.is_null() {
        return 0;
    }

    unsafe {
        match (*handle).ensure_texture(width, height) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

/// 设置纹理
#[no_mangle]
pub extern "C" fn hwcodec_native_device_set_texture(
    handle: NativeDeviceHandle,
    texture: *mut std::ffi::c_void,
) {
    if handle.is_null() || texture.is_null() {
        return;
    }

    unsafe {
        let texture_ptr = texture as *mut ID3D11Texture2D;
        // 创建新的接口引用（不增加引用计数）
        let texture_interface = Interface::from_raw(texture_ptr as *mut std::ffi::c_void);
        // set_texture 会获取所有权，所以我们需要克隆引用
        // 但由于这是外部管理的指针，我们直接传递
        (*handle).set_texture(texture_interface);
        // 不要忘记，因为 set_texture 会获取所有权
    }
}

/// 获取共享句柄
#[no_mangle]
pub extern "C" fn hwcodec_native_device_get_shared_handle(
    handle: NativeDeviceHandle,
) -> HANDLE {
    if handle.is_null() {
        return HANDLE::default();
    }

    unsafe {
        match (*handle).get_shared_handle() {
            Ok(h) => h,
            Err(_) => HANDLE::default(),
        }
    }
}

/// 获取当前纹理
#[no_mangle]
pub extern "C" fn hwcodec_native_device_get_current_texture(
    handle: NativeDeviceHandle,
) -> *mut std::ffi::c_void {
    if handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        (*handle)
            .get_current_texture()
            .map(|t| t.as_raw() as *mut std::ffi::c_void)
            .unwrap_or(ptr::null_mut())
    }
}

/// 移动到下一个纹理
#[no_mangle]
pub extern "C" fn hwcodec_native_device_next(handle: NativeDeviceHandle) -> c_int {
    if handle.is_null() {
        return -1;
    }

    unsafe {
        (*handle).next() as c_int
    }
}

/// 开始查询
#[no_mangle]
pub extern "C" fn hwcodec_native_device_begin_query(handle: NativeDeviceHandle) {
    if !handle.is_null() {
        unsafe {
            (*handle).begin_query();
        }
    }
}

/// 结束查询
#[no_mangle]
pub extern "C" fn hwcodec_native_device_end_query(handle: NativeDeviceHandle) {
    if !handle.is_null() {
        unsafe {
            (*handle).end_query();
        }
    }
}

/// 查询完成状态
#[no_mangle]
pub extern "C" fn hwcodec_native_device_query(handle: NativeDeviceHandle) -> c_int {
    if handle.is_null() {
        return 0;
    }

    unsafe {
        match (*handle).query() {
            Ok(true) => 1,
            Ok(false) => 0,
            Err(_) => 0,
        }
    }
}

/// 获取设备指针（用于访问内部 D3D11 设备）
#[no_mangle]
pub extern "C" fn hwcodec_native_device_get_device(
    handle: NativeDeviceHandle,
) -> *mut std::ffi::c_void {
    if handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        (*handle).device().as_raw() as *mut std::ffi::c_void
    }
}

/// 获取上下文指针
#[no_mangle]
pub extern "C" fn hwcodec_native_device_get_context(
    handle: NativeDeviceHandle,
) -> *mut std::ffi::c_void {
    if handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        (*handle).context().as_raw() as *mut std::ffi::c_void
    }
}

/// 获取视频设备指针
#[no_mangle]
pub extern "C" fn hwcodec_native_device_get_video_device(
    handle: NativeDeviceHandle,
) -> *mut std::ffi::c_void {
    if handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        (*handle).video_device().as_raw() as *mut std::ffi::c_void
    }
}

/// 获取视频上下文指针
#[no_mangle]
pub extern "C" fn hwcodec_native_device_get_video_context(
    handle: NativeDeviceHandle,
) -> *mut std::ffi::c_void {
    if handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        (*handle).video_context().as_raw() as *mut std::ffi::c_void
    }
}

/// 获取视频上下文1指针
#[no_mangle]
pub extern "C" fn hwcodec_native_device_get_video_context1(
    handle: NativeDeviceHandle,
) -> *mut std::ffi::c_void {
    if handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        (*handle).video_context1().as_raw() as *mut std::ffi::c_void
    }
}

/// 获取厂商
#[no_mangle]
pub extern "C" fn hwcodec_native_device_get_vendor(
    handle: NativeDeviceHandle,
) -> c_int {
    if handle.is_null() {
        return common::AdapterVendor::ADAPTER_VENDOR_UNKNOWN as c_int;
    }

    unsafe {
        (*handle).get_vendor() as c_int
    }
}

/// 检查是否支持硬件解码
#[no_mangle]
pub extern "C" fn hwcodec_native_device_support_decode(
    handle: NativeDeviceHandle,
    format: c_int,
) -> c_int {
    if handle.is_null() {
        return 0;
    }

    let data_format = match format {
        0 => common::DataFormat::H264,
        1 => common::DataFormat::H265,
        _ => return 0,
    };

    unsafe {
        match (*handle).support_decode(data_format) {
            Ok(true) => 1,
            Ok(false) => 0,
            Err(_) => 0,
        }
    }
}

// ============================================================================
// Adapters FFI 接口
// ============================================================================

/// 不透明的 Adapters 指针类型
pub type AdaptersHandle = *mut Adapters;

/// 创建新的 Adapters
#[no_mangle]
pub extern "C" fn hwcodec_adapters_new(vendor: c_int) -> AdaptersHandle {
    let adapter_vendor = match vendor {
        0 => common::AdapterVendor::ADAPTER_VENDOR_NVIDIA,
        1 => common::AdapterVendor::ADAPTER_VENDOR_AMD,
        2 => common::AdapterVendor::ADAPTER_VENDOR_INTEL,
        _ => return ptr::null_mut(),
    };

    match Adapters::new(adapter_vendor) {
        Ok(adapters) => Box::into_raw(Box::new(adapters)),
        Err(_) => ptr::null_mut(),
    }
}

/// 释放 Adapters
#[no_mangle]
pub extern "C" fn hwcodec_adapters_destroy(handle: AdaptersHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// 获取第一个适配器索引
#[no_mangle]
pub extern "C" fn hwcodec_adapters_get_first_adapter_index(vendor: c_int) -> c_int {
    let adapter_vendor = match vendor {
        0 => common::AdapterVendor::ADAPTER_VENDOR_NVIDIA,
        1 => common::AdapterVendor::ADAPTER_VENDOR_AMD,
        2 => common::AdapterVendor::ADAPTER_VENDOR_INTEL,
        _ => return -1,
    };

    match Adapters::get_first_adapter_index(adapter_vendor) {
        Ok(index) => index,
        Err(_) => -1,
    }
}

/// 获取适配器数量
#[no_mangle]
pub extern "C" fn hwcodec_adapters_get_count(handle: AdaptersHandle) -> c_int {
    if handle.is_null() {
        return 0;
    }

    unsafe {
        (*handle).adapters().len() as c_int
    }
}

/// 获取指定索引的适配器设备
#[no_mangle]
pub extern "C" fn hwcodec_adapters_get_adapter_device(
    handle: AdaptersHandle,
    index: c_int,
) -> *mut std::ffi::c_void {
    if handle.is_null() || index < 0 {
        return ptr::null_mut();
    }

    unsafe {
        let adapters = (*handle).adapters();
        if index as usize >= adapters.len() {
            return ptr::null_mut();
        }

        adapters[index as usize].device().as_raw() as *mut std::ffi::c_void
    }
}

/// 获取指定索引的适配器描述（用于获取 LUID）
#[no_mangle]
pub extern "C" fn hwcodec_adapters_get_adapter_desc(
    handle: AdaptersHandle,
    index: c_int,
    desc: *mut windows::Win32::Graphics::Dxgi::DXGI_ADAPTER_DESC1,
) -> c_int {
    if handle.is_null() || index < 0 || desc.is_null() {
        return 0;
    }

    unsafe {
        let adapters = (*handle).adapters();
        if index as usize >= adapters.len() {
            return 0;
        }

        *desc = *adapters[index as usize].desc();
        1
    }
}

/// 获取指定索引的适配器 LUID
#[no_mangle]
pub extern "C" fn hwcodec_adapters_get_adapter_luid(
    handle: AdaptersHandle,
    index: c_int,
) -> i64 {
    if handle.is_null() || index < 0 {
        return 0;
    }

    unsafe {
        let adapters = (*handle).adapters();
        if index as usize >= adapters.len() {
            return 0;
        }

        adapters[index as usize].luid()
    }
}

/// 视频处理（NV12 到 NV12 或其他格式转换）
/// 
/// # 参数
/// - `handle`: NativeDevice 句柄
/// - `input`: 输入纹理
/// - `output`: 输出纹理
/// - `width`: 宽度
/// - `height`: 高度
/// - `content_desc`: 视频内容描述结构体指针
/// - `color_space_in`: 输入颜色空间
/// - `color_space_out`: 输出颜色空间
/// - `array_slice`: 数组切片索引
#[no_mangle]
pub extern "C" fn hwcodec_native_device_process(
    handle: NativeDeviceHandle,
    input: *mut std::ffi::c_void,
    output: *mut std::ffi::c_void,
    width: u32,
    height: u32,
    content_desc: *const D3D11_VIDEO_PROCESSOR_CONTENT_DESC,
    color_space_in: DXGI_COLOR_SPACE_TYPE,
    color_space_out: DXGI_COLOR_SPACE_TYPE,
    array_slice: u32,
) -> c_int {
    if handle.is_null() || input.is_null() || output.is_null() || content_desc.is_null() {
        return 0;
    }

    unsafe {
        let input_texture = Interface::from_raw(input as *mut std::ffi::c_void);
        let output_texture = Interface::from_raw(output as *mut std::ffi::c_void);
        let content_desc = *content_desc;

        let result = (*handle).process(
            &input_texture,
            &output_texture,
            width,
            height,
            content_desc,
            color_space_in,
            color_space_out,
            array_slice,
        );

        std::mem::forget(input_texture);
        std::mem::forget(output_texture);

        match result {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

/// 将 BGRA 纹理转换为 NV12
#[no_mangle]
pub extern "C" fn hwcodec_native_device_bgra_to_nv12(
    handle: NativeDeviceHandle,
    bgra_texture: *mut std::ffi::c_void,
    nv12_texture: *mut std::ffi::c_void,
    width: u32,
    height: u32,
    color_space_in: DXGI_COLOR_SPACE_TYPE,
    color_space_out: DXGI_COLOR_SPACE_TYPE,
) -> c_int {
    if handle.is_null() || bgra_texture.is_null() || nv12_texture.is_null() {
        return 0;
    }

    unsafe {
        let bgra = Interface::from_raw(bgra_texture as *mut std::ffi::c_void);
        let nv12 = Interface::from_raw(nv12_texture as *mut std::ffi::c_void);

        let result = (*handle).bgra_to_nv12(&bgra, &nv12, width, height, color_space_in, color_space_out);

        std::mem::forget(bgra);
        std::mem::forget(nv12);

        match result {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

/// 将 NV12 纹理转换为 BGRA
#[no_mangle]
pub extern "C" fn hwcodec_native_device_nv12_to_bgra(
    handle: NativeDeviceHandle,
    nv12_texture: *mut std::ffi::c_void,
    bgra_texture: *mut std::ffi::c_void,
    width: u32,
    height: u32,
    nv12_array_index: u32,
) -> c_int {
    if handle.is_null() || nv12_texture.is_null() || bgra_texture.is_null() {
        return 0;
    }

    unsafe {
        let nv12 = Interface::from_raw(nv12_texture as *mut std::ffi::c_void);
        let bgra = Interface::from_raw(bgra_texture as *mut std::ffi::c_void);

        let result = (*handle).nv12_to_bgra(&nv12, &bgra, width, height, nv12_array_index);

        std::mem::forget(nv12);
        std::mem::forget(bgra);

        match result {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

// ============================================================================
// BMP 和 Dump FFI 接口
// ============================================================================

/// 周期性保存 BGRA 纹理为 BMP 文件
#[no_mangle]
pub extern "C" fn SaveBgraBmps(
    device: *mut std::ffi::c_void,
    texture: *mut std::ffi::c_void,
    cycle: c_int,
) {
    if device.is_null() || texture.is_null() {
        return;
    }

    unsafe {
        let device = Interface::from_raw(device as *mut std::ffi::c_void);
        let texture = Interface::from_raw(texture as *mut std::ffi::c_void);

        let _ = bmp::save_bgra_bmps(&device, Some(&texture), cycle);

        std::mem::forget(device);
        std::mem::forget(texture);
    }
}

/// 转储 NV12 纹理到文件
#[no_mangle]
pub extern "C" fn dumpTexture(
    device: *mut std::ffi::c_void,
    texture: *mut std::ffi::c_void,
    crop_w: c_int,
    crop_h: c_int,
    filename: *const std::os::raw::c_char,
) -> c_int {
    if device.is_null() || texture.is_null() || filename.is_null() {
        return 0;
    }

    unsafe {
        let device = Interface::from_raw(device as *mut std::ffi::c_void);
        let texture = Interface::from_raw(texture as *mut std::ffi::c_void);

        let filename_cstr = std::ffi::CStr::from_ptr(filename);
        let filename_str = match filename_cstr.to_str() {
            Ok(s) => s,
            Err(_) => {
                std::mem::forget(device);
                std::mem::forget(texture);
                return 0;
            }
        };

        let result = dump::dump_texture(
            &device,
            &texture,
            crop_w as u32,
            crop_h as u32,
            filename_str,
        );

        std::mem::forget(device);
        std::mem::forget(texture);

        match result {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}
