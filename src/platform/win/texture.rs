//! 纹理管理工具函数

use crate::platform::win::error::Result;
use windows::Win32::Graphics::Direct3D11::*;

/// 获取 D3D11 纹理的宽度和高度
pub fn get_texture_width_height(texture: &ID3D11Texture2D) -> Result<(u32, u32)> {
    unsafe {
        let mut desc = std::mem::zeroed();
        texture.GetDesc(&mut desc);
        Ok((desc.Width, desc.Height))
    }
}
