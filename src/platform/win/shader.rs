//! 着色器管理
//! 
//! 提供 NV12 到 BGRA 转换所需的顶点和像素着色器

use crate::platform::win::error::Result;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;

// 着色器字节码（从 C++ 头文件提取）
const VERTEX_SHADER_BYTECODE: &[u8] = include_bytes!("../../../shaders/vertex_shader.bin");
const PIXEL_SHADER_BYTECODE: &[u8] = include_bytes!("../../../shaders/pixel_shader_601.bin");

/// 创建顶点着色器
pub fn create_vertex_shader(
    device: &ID3D11Device,
) -> Result<ID3D11VertexShader> {
    unsafe {
        let mut shader = None;
        device.CreateVertexShader(
            VERTEX_SHADER_BYTECODE,
            None,
            Some(&mut shader),
        )?;
        Ok(shader.unwrap())
    }
}

/// 创建像素着色器
pub fn create_pixel_shader(
    device: &ID3D11Device,
) -> Result<ID3D11PixelShader> {
    unsafe {
        let mut shader = None;
        device.CreatePixelShader(
            PIXEL_SHADER_BYTECODE,
            None,
            Some(&mut shader),
        )?;
        Ok(shader.unwrap())
    }
}

/// 创建输入布局
pub fn create_input_layout(
    device: &ID3D11Device,
) -> Result<ID3D11InputLayout> {
    use windows::core::PCSTR;
    
    let layout_desc = [
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: PCSTR::from_raw(b"POSITION\0".as_ptr() as *const u8),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32B32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: PCSTR::from_raw(b"TEXCOORD\0".as_ptr() as *const u8),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 12,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
    ];

    unsafe {
        let mut layout = None;
        device.CreateInputLayout(
            &layout_desc,
            VERTEX_SHADER_BYTECODE,
            Some(&mut layout),
        )?;
        Ok(layout.unwrap())
    }
}
