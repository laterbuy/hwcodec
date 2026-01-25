//! Windows 平台 DirectX 11 实现
//! 
//! 本模块提供 Windows 平台下的 D3D11 硬件加速视频编解码支持

pub mod error;
pub mod adapter;
pub mod device;
pub mod texture;
pub mod video_processor;
pub mod format_conversion;
pub mod shader;
pub mod bmp;
pub mod dump;
pub mod utils;

// FFI 接口仅在 Windows 平台导出
#[cfg(windows)]
pub mod ffi;

// 重新导出主要类型
pub use adapter::{Adapter, Adapters};
pub use device::NativeDevice;
pub use error::WinPlatformError;

// 重新导出工具函数用于测试
pub use utils::{get_gpu_signature, add_process_to_new_job};

#[cfg(test)]
mod tests {
    use super::*;
    
    /// 测试适配器枚举（需要 GPU）
    #[test]
    #[ignore] // 需要 GPU，默认忽略
    fn test_adapter_enumeration() {
        // 这个测试需要实际的 GPU
        // 在 CI/CD 环境中可能无法运行
        let adapters = Adapters::new(crate::common::AdapterVendor::ADAPTER_VENDOR_NVIDIA);
        match adapters {
            Ok(adapters) => {
                println!("Found {} NVIDIA adapters", adapters.adapters().len());
            }
            Err(e) => {
                println!("Failed to enumerate adapters: {}", e);
            }
        }
    }
    
    /// 测试 GPU 签名计算
    #[test]
    fn test_gpu_signature() {
        let signature = utils::get_gpu_signature();
        // GPU 签名应该是一个非零值（如果有 GPU）
        // 如果没有 GPU，可能是 0
        println!("GPU Signature: {}", signature);
    }
}
