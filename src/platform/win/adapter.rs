//! GPU 适配器管理

use crate::common;
use crate::platform::win::error::{Result, WinPlatformError};
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::*;

/// GPU 适配器
pub struct Adapter {
    #[allow(dead_code)]
    adapter: IDXGIAdapter1,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    desc: DXGI_ADAPTER_DESC1,
}

impl Adapter {
    /// 从 IDXGIAdapter1 初始化适配器
    pub fn new(adapter: IDXGIAdapter1) -> Result<Self> {
        let desc = unsafe { adapter.GetDesc1()? };

        // 创建 D3D11 设备
        let feature_levels = [D3D_FEATURE_LEVEL_11_0];
        let mut device = None;
        let mut context = None;
        let mut feature_level = D3D_FEATURE_LEVEL(0);

        unsafe {
            // D3D11CreateDevice需要IDXGIAdapter，需要转换
            let adapter_base: IDXGIAdapter = Interface::cast(&adapter)?;
            D3D11CreateDevice(
                &adapter_base,
                D3D_DRIVER_TYPE_UNKNOWN,
                windows::Win32::Foundation::HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG::default(),
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut device),
                Some(&mut feature_level),
                Some(&mut context),
            )?;
        }

        let device = device.unwrap();
        let context = context.unwrap();

        if feature_level != D3D_FEATURE_LEVEL_11_0 {
            return Err(WinPlatformError::UnsupportedFeatureLevel(feature_level));
        }

        // 对于 Intel GPU，启用多线程保护
        if desc.VendorId == common::AdapterVendor::ADAPTER_VENDOR_INTEL as u32 {
            let adapter = Self {
                adapter,
                device,
                context,
                desc,
            };
            adapter.set_multithread_protected()?;
            Ok(adapter)
        } else {
            Ok(Self {
                adapter,
                device,
                context,
                desc,
            })
        }
    }

    /// 设置多线程保护
    fn set_multithread_protected(&self) -> Result<()> {
        unsafe {
            let multithread: windows::Win32::Graphics::Direct3D10::ID3D10Multithread =
                windows::core::Interface::cast(&self.context)?;
            let _ = multithread.SetMultithreadProtected(true);
            Ok(())
        }
    }

    /// 获取设备
    pub fn device(&self) -> &ID3D11Device {
        &self.device
    }

    /// 获取上下文
    pub fn context(&self) -> &ID3D11DeviceContext {
        &self.context
    }

    /// 获取适配器描述
    pub fn desc(&self) -> &DXGI_ADAPTER_DESC1 {
        &self.desc
    }

    /// 获取 LUID
    pub fn luid(&self) -> i64 {
        ((self.desc.AdapterLuid.HighPart as i64) << 32)
            | self.desc.AdapterLuid.LowPart as i64
    }

    /// 获取厂商 ID
    pub fn vendor_id(&self) -> u32 {
        self.desc.VendorId
    }

    /// 获取设备 ID
    pub fn device_id(&self) -> u32 {
        self.desc.DeviceId
    }
}

/// 适配器集合
pub struct Adapters {
    #[allow(dead_code)]
    factory: IDXGIFactory1,
    adapters: Vec<Adapter>,
}

impl Adapters {
    /// 创建适配器集合，枚举指定厂商的所有适配器
    pub fn new(vendor: common::AdapterVendor) -> Result<Self> {
        let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };

        let mut adapters = Vec::new();
        let mut adapter_index = 0;

        loop {
            let adapter: IDXGIAdapter1 = unsafe {
                match factory.EnumAdapters1(adapter_index) {
                    Ok(a) => a,
                    Err(_) => break, // 枚举结束
                }
            };

            let desc = unsafe { adapter.GetDesc1()? };
            if desc.VendorId == vendor as u32 {
                match Adapter::new(adapter) {
                    Ok(adapter) => adapters.push(adapter),
                    Err(e) => {
                        log::warn!("Failed to initialize adapter {}: {}", adapter_index, e);
                    }
                }
            }

            adapter_index += 1;
        }

        Ok(Self { factory, adapters })
    }

    /// 获取适配器列表
    pub fn adapters(&self) -> &[Adapter] {
        &self.adapters
    }

    /// 获取第一个适配器索引（全局索引，不是集合中的索引）
    pub fn get_first_adapter_index(vendor: common::AdapterVendor) -> Result<i32> {
        let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };

        let mut adapter_index = 0;
        loop {
            let adapter: IDXGIAdapter1 = unsafe {
                match factory.EnumAdapters1(adapter_index) {
                    Ok(a) => a,
                    Err(_) => return Err(WinPlatformError::AdapterNotFound),
                }
            };

            let desc = unsafe { adapter.GetDesc1()? };
            if desc.VendorId == vendor as u32 {
                return Ok(adapter_index as i32);
            }

            adapter_index += 1;
        }
    }
}
