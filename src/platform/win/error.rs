//! Windows 平台错误类型定义

use thiserror::Error;
use windows::core::HRESULT;

/// Windows 平台错误类型
#[derive(Error, Debug)]
pub enum WinPlatformError {
    /// D3D11 设备创建失败
    #[error("D3D11 device creation failed: {0}")]
    DeviceCreationFailed(#[from] windows::core::Error),

    /// 适配器未找到
    #[error("Adapter not found")]
    AdapterNotFound,

    /// 不支持的特性级别
    #[error("Unsupported feature level: {0:?}")]
    UnsupportedFeatureLevel(windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL),

    /// 纹理创建失败
    #[error("Texture creation failed: {0}")]
    TextureCreationFailed(HRESULT),

    /// 视频处理器创建失败
    #[error("Video processor creation failed: {0}")]
    VideoProcessorCreationFailed(HRESULT),

    /// 着色器创建失败
    #[error("Shader creation failed: {0}")]
    ShaderCreationFailed(HRESULT),

    /// 资源映射失败
    #[error("Resource mapping failed: {0}")]
    ResourceMappingFailed(HRESULT),

    /// 文件操作失败
    #[error("File operation failed: {0}")]
    FileOperationFailed(#[from] std::io::Error),

    /// 无效参数
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// 不支持的格式
    #[error("Unsupported format")]
    UnsupportedFormat,

    /// 多线程保护设置失败
    #[error("Failed to set multithread protection")]
    MultithreadProtectionFailed,
}

/// Result 类型别名
pub type Result<T> = std::result::Result<T, WinPlatformError>;

/// 从 HRESULT 转换为 Result
pub fn check_hresult(hr: HRESULT) -> Result<()> {
    if hr.is_ok() {
        Ok(())
    } else {
        Err(WinPlatformError::DeviceCreationFailed(windows::core::Error::from(hr)))
    }
}
