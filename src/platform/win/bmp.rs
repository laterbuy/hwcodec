//! BMP 文件保存功能
//! 
//! 将 BGRA 纹理保存为 BMP 文件

use crate::platform::win::error::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;

/// 创建 24 位 BMP 文件
fn create_bmp_file(
    filename: &Path,
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<()> {
    let file_size = (width * height * 3 + 54) as u32;

    // BMP 文件头（54 字节）
    let mut header = [0u8; 54];
    header[0] = 0x42; // 'B'
    header[1] = 0x4D; // 'M'
    
    // 文件大小（小端）
    header[2] = file_size as u8;
    header[3] = (file_size >> 8) as u8;
    header[4] = (file_size >> 16) as u8;
    header[5] = (file_size >> 24) as u8;
    
    header[10] = 54; // 数据偏移
    
    // DIB 头
    header[14] = 40; // DIB 头大小
    header[18] = width as u8;
    header[19] = (width >> 8) as u8;
    header[20] = (width >> 16) as u8;
    header[21] = (width >> 24) as u8;
    header[22] = height as u8;
    header[23] = (height >> 8) as u8;
    header[24] = (height >> 16) as u8;
    header[25] = (height >> 24) as u8;
    header[26] = 1; // 颜色平面数
    header[28] = 24; // 每像素位数
    
    let image_size = (width * height * 3) as u32;
    header[34] = image_size as u8;
    header[35] = (image_size >> 8) as u8;
    header[36] = (image_size >> 16) as u8;
    header[37] = (image_size >> 24) as u8;

    let mut file = File::create(filename)?;
    file.write_all(&header)?;

    // BMP 格式要求从下到上存储
    let stride = (width * 3) as usize;
    for row in (0..height as usize).rev() {
        let start = row * stride;
        let end = start + stride;
        file.write_all(&data[start..end])?;
    }

    Ok(())
}

/// 从 BGRA 纹理创建 BMP 文件
pub fn create_bgra_bmp_file(
    device: &ID3D11Device,
    texture: &ID3D11Texture2D,
    filename: &str,
) -> Result<()> {
    // 获取纹理描述
    let desc = unsafe {
        let mut desc = std::mem::zeroed();
        texture.GetDesc(&mut desc);
        desc
    };

    // 创建 staging 纹理（CPU 可读）
    let staging_desc = D3D11_TEXTURE2D_DESC {
        Width: desc.Width,
        Height: desc.Height,
        MipLevels: 1,
        ArraySize: 1,
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

    // 转换为 BGR（去除 Alpha 通道）
    let image_size = (desc.Width * desc.Height * 3) as usize;
    let mut bgr_data = Vec::with_capacity(image_size);

    unsafe {
        let rgba_data = std::slice::from_raw_parts(
            mapped_resource.pData as *const u8,
            (desc.Height * mapped_resource.RowPitch) as usize,
        );

        for row in 0..desc.Height as usize {
            let row_start = row * mapped_resource.RowPitch as usize;
            for col in 0..desc.Width as usize {
                let pixel_start = row_start + col * 4;
                if desc.Format == DXGI_FORMAT_B8G8R8A8_UNORM {
                    // BGRA -> BGR
                    bgr_data.push(rgba_data[pixel_start]);     // B
                    bgr_data.push(rgba_data[pixel_start + 1]); // G
                    bgr_data.push(rgba_data[pixel_start + 2]); // R
                } else {
                    // RGBA -> BGR
                    bgr_data.push(rgba_data[pixel_start + 2]); // B
                    bgr_data.push(rgba_data[pixel_start + 1]); // G
                    bgr_data.push(rgba_data[pixel_start]);     // R
                }
            }
        }
    }

    // 确保目录存在
    let path = Path::new(filename);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 写入 BMP 文件
    create_bmp_file(path, &bgr_data, desc.Width, desc.Height)?;

    // 取消映射
    unsafe {
        context.Unmap(&staging_texture, 0);
    }

    Ok(())
}

/// 周期性保存 BGRA 纹理为 BMP 文件
pub fn save_bgra_bmps(
    device: &ID3D11Device,
    texture: Option<&ID3D11Texture2D>,
    cycle: i32,
) -> Result<()> {
    if texture.is_none() {
        return Ok(());
    }

    static mut INDEX: i32 = 0;
    unsafe {
        let index = INDEX;
        if index % cycle == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let time_str = format!("{:08X}", now);
            let filename = format!("bmps/{}_{}.bmp", index, time_str);
            create_bgra_bmp_file(device, texture.unwrap(), &filename)?;
        }
        INDEX += 1;
    }

    Ok(())
}
