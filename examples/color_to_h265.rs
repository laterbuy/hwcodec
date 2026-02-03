//! Demo: 创建色彩随时间变化的纹理，编码为 H.265 (HEVC) 并保存到文件。
//!
//! 运行: cargo run --example color_to_h265
//! 输出: output/color_demo.h265

use env_logger::{init_from_env, Env, DEFAULT_FILTER_ENV};
use hwcodec::common::{DataFormat::H265, Driver, MAX_GOP};
use hwcodec::vram::{encode, DynamicContext, EncodeContext};
use std::fs::File;
use std::io::Write;
use std::os::raw::c_void;

#[cfg(windows)]
use windows::{
    core::*,
    Win32::Foundation::{E_FAIL, HMODULE},
    Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0},
    Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, D3D11_SDK_VERSION, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        D3D11_CREATE_DEVICE_VIDEO_SUPPORT, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_WRITE,
        D3D11_MAP_WRITE, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
        D3D11_USAGE_STAGING, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    },
    Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
    Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIAdapter, IDXGIFactory1},
};

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 720;
const FRAMERATE: i32 = 30;
const DURATION_SEC: i32 = 4;
const OUTPUT_PATH: &str = "output/color_demo.h265";

#[cfg(windows)]
fn main() {
    init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));

    let total_frames = FRAMERATE * DURATION_SEC;
    log::info!(
        "Demo: 色彩变化纹理 -> H.265 (HEVC), {}x{}, {} fps, {} 秒, {} 帧",
        WIDTH, HEIGHT, FRAMERATE, DURATION_SEC, total_frames
    );

    let (device, texture) = match create_d3d11_device_and_texture(WIDTH, HEIGHT) {
        Ok(r) => r,
        Err(e) => {
            log::error!("创建 D3D11 设备失败: {:?}", e);
            return;
        }
    };

    let device_ptr = device.as_raw() as *mut c_void;
    log::debug!("D3D11 设备指针: {:p} (纹理与编码器须使用同一设备)", device_ptr);

    let dynamic_ctx = DynamicContext {
        device: Some(device_ptr),
        width: WIDTH,
        height: HEIGHT,
        kbitrate: 4000,
        framerate: FRAMERATE,
        gop: MAX_GOP as i32,
    };

    let available = encode::available(dynamic_ctx.clone());
    log::info!(
        "检测到 {} 个可用编码器: {}",
        available.len(),
        available
                            .iter()
                            .map(|e| format!("{:?}/{:?}", e.driver, e.data_format))
                            .collect::<Vec<_>>()
                            .join(", ")
    );

    let encoder_feature = match available
        .iter()
        .find(|e| e.data_format == H265 && e.driver == Driver::NV)
        .or_else(|| available.iter().find(|e| e.data_format == H265))
    {
        Some(e) => {
            log::info!("使用 H.265 编码器: {:?}", e.driver);
            e.clone()
        }
        None => {
            log::error!("未找到 H.265 编码器");
            log::error!("请确认：1) 驱动可用 (NVIDIA/AMD/Intel)；2) 纹理与编码器须使用同一 D3D11 设备");
            if available.is_empty() {
                log::error!("当前无任何编码器通过检测 (无驱动报告 H.264/H.265 或 test 未通过)");
                log::error!("可设置 RUST_LOG=debug 查看编码器候选与各驱动 test 结果");
            } else {
                log::error!("有 {} 个编码器可用但均非 H.265，当前仅请求 H.265", available.len());
            }
            return;
        }
    };

    let mut encode_ctx = EncodeContext {
        f: encoder_feature,
        d: dynamic_ctx,
    };
    encode_ctx.d.device = Some(device_ptr);

    let mut encoder = match encode::Encoder::new(encode_ctx) {
        Ok(enc) => enc,
        Err(_) => {
            log::error!("创建 H.265 编码器失败");
            log::error!("请确认纹理与编码器使用同一 D3D11 设备 (本 demo 已使用同一 device 创建纹理与编码器)");
            return;
        }
    };

    if let Err(e) = std::fs::create_dir_all("output") {
        log::error!("创建 output 目录失败: {:?}", e);
        return;
    }

    let mut file = match File::create(OUTPUT_PATH) {
        Ok(f) => f,
        Err(e) => {
            log::error!("创建输出文件失败 {}: {:?}", OUTPUT_PATH, e);
            return;
        }
    };

    let context = unsafe { device.GetImmediateContext().unwrap() };
    let frame_duration_ms = 1000i64 / FRAMERATE as i64;

    log::info!("编码中: {} 帧 -> {}", total_frames, OUTPUT_PATH);

    for frame_num in 0..total_frames {
        fill_texture_color_cycle(&device, &context, &texture, WIDTH, HEIGHT, frame_num, total_frames);
        unsafe { context.Flush() };

        let pts = frame_num as i64 * frame_duration_ms;
        let texture_ptr = texture.as_raw() as *mut c_void;

        match encoder.encode(texture_ptr, pts) {
            Ok(frames) => {
                for f in frames.iter() {
                    let _ = file.write_all(&f.data);
                }
                if frame_num % 30 == 0 || frame_num < 3 {
                    let bytes: usize = frames.iter().map(|f| f.data.len()).sum();
                    log::info!("  帧 {}/{} ({} bytes)", frame_num + 1, total_frames, bytes);
                }
            }
            Err(code) => {
                log::error!("编码失败 帧 {} code={}", frame_num, code);
                if frame_num == 0 {
                    log::error!("首帧失败可能原因: 纹理格式/编码器初始化问题");
                    if encoder.ctx.f.driver != Driver::NV {
                        log::error!(
                            "AMD：确保 externals/AMF_v1.4.35 存在并重新构建；Intel：安装 Media SDK 运行时（mfx.dll）；或使用 NVIDIA GPU。"
                        );
                        log::error!("若 AMF 仍失败，请查看 stderr 中 [AMF] 调试日志。");
                    }
                }
                return;
            }
        }
    }

    log::info!("完成: 已保存 {}", OUTPUT_PATH);
}

/// 创建 D3D11 设备和 BGRA 纹理
#[cfg(windows)]
fn create_d3d11_device_and_texture(
    width: i32,
    height: i32,
) -> std::result::Result<(ID3D11Device, ID3D11Texture2D), Error> {
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1()?;
        let adapter1 = factory.EnumAdapters1(0).map_err(|_| Error::from(E_FAIL))?;
        let adapter: IDXGIAdapter = adapter1.cast()?;

        let mut device: Option<ID3D11Device> = None;
        let mut _ctx: Option<ID3D11DeviceContext> = None;
        let feature_levels = [D3D_FEATURE_LEVEL_11_0];
        let mut _level = D3D_FEATURE_LEVEL_11_0;

        D3D11CreateDevice(
            Some(&adapter),
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_VIDEO_SUPPORT | D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut _level),
            Some(&mut _ctx),
        )?;

        let device = device.ok_or(Error::from(E_FAIL))?;

        let desc = D3D11_TEXTURE2D_DESC {
            Width: width as u32,
            Height: height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: Default::default(),
        };

        let mut texture: Option<ID3D11Texture2D> = None;
        device.CreateTexture2D(&desc, None, Some(&mut texture))?;
        let texture = texture.ok_or(Error::from(E_FAIL))?;

        Ok((device, texture))
    }
}

/// 用“色彩循环”填充纹理
#[cfg(windows)]
fn fill_texture_color_cycle(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    width: i32,
    height: i32,
    frame_num: i32,
    total_frames: i32,
) {
    unsafe {
        let staging_desc = D3D11_TEXTURE2D_DESC {
            Width: width as u32,
            Height: height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            MiscFlags: Default::default(),
        };

        let mut staging: Option<ID3D11Texture2D> = None;
        if device.CreateTexture2D(&staging_desc, None, Some(&mut staging)).is_err() {
            return;
        }
        let staging = match staging {
            Some(t) => t,
            None => return,
        };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        if context
            .Map(&staging, 0, D3D11_MAP_WRITE, 0, Some(&mut mapped))
            .is_err()
        {
            return;
        }

        let row_pitch = mapped.RowPitch as usize;
        let data = std::slice::from_raw_parts_mut(
            mapped.pData as *mut u8,
            row_pitch * height as usize,
        );

        let t = (frame_num as f32 / total_frames.max(1) as f32) * 3.0_f32;
        let phase = t.fract();

        for y in 0..height {
            for x in 0..width {
                let offset = (y * row_pitch as i32 + x * 4) as usize;
                if offset + 3 >= data.len() {
                    continue;
                }
                let xr = x as f32 / width.max(1) as f32;
                let yr = y as f32 / height.max(1) as f32;
                let edge = (xr * (1.0 - xr) * 4.0).min(1.0) * (yr * (1.0 - yr) * 4.0).min(1.0);
                let (r, g, b) = hue_cycle_to_rgb(phase, xr, yr);
                let r = (r * 255.0 * edge + 30.0) as u8;
                let g = (g * 255.0 * edge + 30.0) as u8;
                let b = (b * 255.0 * edge + 30.0) as u8;
                data[offset] = b;
                data[offset + 1] = g;
                data[offset + 2] = r;
                data[offset + 3] = 255;
            }
        }

        context.Unmap(&staging, 0);
        context.CopyResource(texture, &staging);
    }
}

#[cfg(windows)]
fn hue_cycle_to_rgb(phase: f32, x_ratio: f32, y_ratio: f32) -> (f32, f32, f32) {
    let hue = phase + (x_ratio + y_ratio) * 0.2;
    let hue = hue.fract();
    let h = hue * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let (r, g, b) = match i % 6 {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

#[cfg(not(windows))]
fn main() {
    println!("此 demo 仅支持 Windows (D3D11 + 硬件编码)");
}
