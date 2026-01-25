//! 工具函数

use crate::common;
use crate::platform::win::error::Result;
use windows::core::Interface;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::System::Threading::*;
use windows::Win32::System::JobObjects::*;
use windows::Win32::Foundation::HANDLE;

// FFI 绑定 CreateJobObjectW
#[link(name = "kernel32")]
extern "system" {
    fn CreateJobObjectW(
        lpjobattributes: *const windows::Win32::Security::SECURITY_ATTRIBUTES,
        lpname: *const u16,
    ) -> HANDLE;
}

/// 计算 GPU 签名（用于检测 GPU 驱动更新）
pub fn get_gpu_signature() -> u64 {
    let factory: IDXGIFactory1 = match unsafe { CreateDXGIFactory1() } {
        Ok(f) => f,
        Err(_) => return 0,
    };

    let mut signature = 0u64;
    let mut adapter_index = 0;

    loop {
        let adapter: IDXGIAdapter1 = unsafe {
            match factory.EnumAdapters1(adapter_index) {
                Ok(a) => a,
                Err(_) => break,
            }
        };

        let desc = match unsafe { adapter.GetDesc1() } {
            Ok(d) => d,
            Err(_) => {
                adapter_index += 1;
                continue;
            }
        };

        if desc.VendorId == common::AdapterVendor::ADAPTER_VENDOR_NVIDIA as u32
            || desc.VendorId == common::AdapterVendor::ADAPTER_VENDOR_AMD as u32
            || desc.VendorId == common::AdapterVendor::ADAPTER_VENDOR_INTEL as u32
        {
            signature = signature
                .wrapping_add(desc.VendorId as u64)
                .wrapping_add(desc.DeviceId as u64)
                .wrapping_add(desc.SubSysId as u64)
                .wrapping_add(desc.Revision as u64);

            // 获取 UMD 版本
            unsafe {
                let guid = <IDXGIDevice as Interface>::IID;
                if let Ok(umd_version) = adapter.CheckInterfaceSupport(&guid) {
                    signature = signature.wrapping_add(umd_version as u64);
                }
            }
        }

        adapter_index += 1;
    }

    signature
}

/// 将进程添加到新的作业对象
/// 当作业对象关闭时，子进程会自动终止
pub fn add_process_to_new_job(process_id: u32) -> Result<()> {
    unsafe {
        let job_handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job_handle.is_invalid() {
            return Err(crate::platform::win::error::WinPlatformError::DeviceCreationFailed(
                windows::core::Error::from_thread(),
            ));
        }

        let mut job_info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        job_info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        SetInformationJobObject(
            job_handle,
            JobObjectExtendedLimitInformation,
            &job_info as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .map_err(|e| crate::platform::win::error::WinPlatformError::DeviceCreationFailed(e))?;

        let process_handle = OpenProcess(
            PROCESS_SET_QUOTA | PROCESS_TERMINATE,
            false,
            process_id,
        )
        .map_err(|e| crate::platform::win::error::WinPlatformError::DeviceCreationFailed(e))?;

        AssignProcessToJobObject(job_handle, process_handle)
            .map_err(|e| crate::platform::win::error::WinPlatformError::DeviceCreationFailed(e))?;

        // 关闭进程句柄，但保留作业对象句柄
        // 作业对象会在进程退出时自动清理
        Ok(())
    }
}
