//! Windows 平台功能测试
//! 
//! 这些测试需要 Windows 平台和可用的 GPU

#[cfg(windows)]
mod tests {
    use hwcodec::common;
    use hwcodec::platform::win;

    /// 测试适配器枚举
    #[test]
    #[ignore] // 需要 GPU，默认忽略
    fn test_adapter_enumeration() {
        // 测试 NVIDIA 适配器
        let nvidia_adapters = win::Adapters::new(common::AdapterVendor::ADAPTER_VENDOR_NVIDIA);
        if let Ok(adapters) = nvidia_adapters {
            println!("Found {} NVIDIA adapters", adapters.adapters().len());
        }

        // 测试 AMD 适配器
        let amd_adapters = win::Adapters::new(common::AdapterVendor::ADAPTER_VENDOR_AMD);
        if let Ok(adapters) = amd_adapters {
            println!("Found {} AMD adapters", adapters.adapters().len());
        }

        // 测试 Intel 适配器
        let intel_adapters = win::Adapters::new(common::AdapterVendor::ADAPTER_VENDOR_INTEL);
        if let Ok(adapters) = intel_adapters {
            println!("Found {} Intel adapters", adapters.adapters().len());
        }
    }

    /// 测试设备创建（从 LUID）
    #[test]
    #[ignore] // 需要 GPU，默认忽略
    fn test_device_creation_from_luid() {
        // 获取第一个 NVIDIA 适配器的索引
        let adapter_index = win::Adapters::get_first_adapter_index(
            common::AdapterVendor::ADAPTER_VENDOR_NVIDIA,
        );
        
        if let Ok(_index) = adapter_index {
            // 这里需要实际的 LUID，暂时跳过
            // let device = win::NativeDevice::new(luid, None, 1);
            println!("Adapter index found: {}", _index);
        }
    }

    /// 测试 GPU 签名计算
    #[test]
    fn test_gpu_signature() {
        let signature = hwcodec::platform::win::get_gpu_signature();
        println!("GPU Signature: 0x{:016X}", signature);
        // 签名可能是 0（如果没有 GPU），这是正常的
    }

    /// 测试 FFI 函数
    #[test]
    fn test_ffi_functions() {
        // 测试 GPU 签名 FFI（通过 common 模块）
        let signature = hwcodec::common::get_gpu_signature();
        println!("FFI GPU Signature: 0x{:016X}", signature);
    }
}
