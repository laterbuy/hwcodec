//! 纹理转储功能
//! 
//! 将 NV12 格式纹理转储到文件

use crate::platform::win::error::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;

/// 转储 NV12 纹理到文件
pub fn dump_texture(
    device: &ID3D11Device,
    texture: &ID3D11Texture2D,
    crop_w: u32,
    crop_h: u32,
    filename: &str,
) -> Result<()> {
    // 确保目录存在
    let dir = Path::new("texture");
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }

    // 获取纹理描述
    let desc = unsafe {
        let mut desc = std::mem::zeroed();
        texture.GetDesc(&mut desc);
        desc
    };

    // 创建 staging 纹理
    let staging_desc = D3D11_TEXTURE2D_DESC {
        Width: desc.Width,
        Height: desc.Height,
        MipLevels: 1,
        ArraySize: desc.ArraySize,
        Format: desc.Format,
        SampleDesc: desc.SampleDesc,
        Usage: D3D11_USAGE_STAGING,
        BindFlags: Default::default(),
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        MiscFlags: Default::default(),
    };

    let staging_texture = unsafe {
        let mut texture = None;
        device.CreateTexture2D(&staging_desc, None, Some(&mut texture))?;
        texture.unwrap()
    };

    // 复制纹理数据
    let context = unsafe { device.GetImmediateContext()? };

    unsafe {
        context.CopyResource(&staging_texture, texture);
    }

    // 映射到 CPU 内存
    let mapped_resource = unsafe {
        let mut mapped = std::mem::zeroed();
        context.Map(&staging_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
        mapped
    };

    // 写入文件
    let path = dir.join(filename);
    let mut file = File::create(path)?;

    if desc.Format == DXGI_FORMAT_NV12 {
        unsafe {
            let data = std::slice::from_raw_parts(
                mapped_resource.pData as *const u8,
                (desc.Height * mapped_resource.RowPitch) as usize,
            );

            let pitch = mapped_resource.RowPitch as usize;
            let y_plane = data;
            let uv_offset = (desc.Height * mapped_resource.RowPitch) as usize;
            let uv_plane = &data[uv_offset..];

            // 写入 Y 平面
            for row in 0..crop_h as usize {
                let start = row * pitch;
                let end = start + crop_w as usize;
                file.write_all(&y_plane[start..end])?;
            }

            // 写入 UV 平面（交错存储）
            let chroma_h = (crop_h / 2) as usize;
            for row in 0..chroma_h {
                let start = row * pitch;
                let end = start + crop_w as usize;
                file.write_all(&uv_plane[start..end])?;
            }
        }
    }

    // 取消映射
    unsafe {
        context.Unmap(&staging_texture, 0);
    }

    Ok(())
}
