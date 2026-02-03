//! D3D11 视频处理器封装
//!
//! 视频处理功能在 device.rs 中实现，本模块提供类型占位以便扩展。

/// 视频处理器标记类型；实际处理通过 `NativeDevice` 与 D3D11 管线完成。
#[derive(Debug, Clone, Copy, Default)]
pub struct VideoProcessor;
