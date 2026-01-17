use env_logger::{init_from_env, Env, DEFAULT_FILTER_ENV};
use hwcodec::common::{DataFormat::H264, MAX_GOP};
use hwcodec::vram::{encode, DynamicContext, EncodeContext};
use std::fs::File;
use std::io::Write;
use std::os::raw::c_void;
use std::time::{Duration, Instant};

#[cfg(windows)]
use windows::{
    core::*,
    Win32::Foundation::{E_FAIL, HMODULE},
    Win32::Graphics::Direct3D::{
        D3D_FEATURE_LEVEL_11_0, D3D_DRIVER_TYPE_UNKNOWN,
    },
    Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, D3D11_SDK_VERSION,
        D3D11_CREATE_DEVICE_VIDEO_SUPPORT, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_WRITE,
        D3D11_MAP_WRITE, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
        D3D11_TEXTURE2D_DESC, D3D11_MAPPED_SUBRESOURCE,
    },
    Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
    Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC,
    Win32::Graphics::Dxgi::{IDXGIAdapter, *},
};

#[cfg(windows)]
fn main() {
    init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
    
    let width = 1920;
    let height = 1080;
    let framerate = 30;
    let duration_seconds = 5;
    let total_frames = framerate * duration_seconds;
    
    log::info!("Creating D3D11 device...");
    let (device, texture) = match create_d3d11_device_and_texture(width, height) {
        Ok(result) => result,
        Err(e) => {
            log::error!("Failed to create D3D11 device: {:?}", e);
            return;
        }
    };
    
    log::info!("Finding available encoders...");
    // Try without device first (like available.rs example)
    let dynamic_ctx = DynamicContext {
        device: None,
        width,
        height,
        kbitrate: 5000,
        framerate,
        gop: MAX_GOP as i32,
    };
    
    // Removed debug logging as requested
    let available_encoders = encode::available(dynamic_ctx.clone());
    log::info!("Found {} available encoders", available_encoders.len());
    
    // If no encoders found through testing, try to create one manually using decoder info
    let encoder_feature = if available_encoders.is_empty() {
        log::warn!("No encoders found through testing, trying to create encoder manually...");
        
        // Get decoder info to find a LUID
        use hwcodec::vram::decode;
        let decoders = decode::available();
        log::info!("Found {} decoders", decoders.len());
        
        // Find H264 decoder to get LUID
        let h264_decoder = decoders.iter()
            .find(|d| d.data_format == H264);
        
        match h264_decoder {
            Some(dec) => {
                log::info!("Using decoder LUID: {} for encoder", dec.luid);
                use hwcodec::common::Driver::AMF;
                use hwcodec::vram::FeatureContext;
                FeatureContext {
                    driver: AMF,
                    vendor: AMF,
                    luid: dec.luid,
                    data_format: H264,
                }
            }
            None => {
                log::error!("No H264 decoder found either!");
                return;
            }
        }
    } else {
        // Find H264 encoder from available encoders
        let h264_encoder = available_encoders
            .iter()
            .find(|e| e.data_format == H264);
        
        match h264_encoder {
            Some(e) => {
                log::info!("Found H264 encoder: {:?}", e);
                e.clone()
            }
            None => {
                log::error!("No H264 encoder found! Available encoders:");
                for enc in &available_encoders {
                    log::error!("  - {:?}", enc);
                }
                return;
            }
        }
    };
    
    // Update context with device for actual encoding
    let mut encode_ctx = EncodeContext {
        f: encoder_feature,
        d: dynamic_ctx,
    };
    // Set device for encoding (not needed for discovery, but needed for encoding)
    let device_ptr = device.as_raw() as *mut c_void;
    encode_ctx.d.device = Some(device_ptr);
    
    log::info!("Creating encoder...");
    let mut encoder = match encode::Encoder::new(encode_ctx) {
        Ok(enc) => enc,
        Err(_) => {
            log::error!("Failed to create encoder");
            return;
        }
    };
    
    log::info!("Creating output directory...");
    if let Err(e) = std::fs::create_dir_all("output") {
        log::error!("Failed to create output directory: {:?}", e);
        return;
    }
    
    log::info!("Opening output file...");
    let output_path = "output/output.h264";
    let mut output_file = match File::create(output_path) {
        Ok(f) => f,
        Err(e) => {
            log::error!("Failed to create output file: {:?}", e);
            return;
        }
    };
    
    log::info!("Encoding {} frames...", total_frames);
    let start_time = Instant::now();
    let frame_duration_ms = 1000i64 / framerate as i64;
    
    // Get device context
    let context = unsafe { 
        match device.GetImmediateContext() {
            Ok(ctx) => ctx,
            Err(e) => {
                log::error!("Failed to get device context: {:?}", e);
                return;
            }
        }
    };
    
    for frame_num in 0..total_frames {
        // Generate test texture pattern
        generate_test_texture(&device, &context, &texture, width, height, frame_num);
        
        // Calculate PTS (presentation timestamp) in milliseconds
        let pts = frame_num as i64 * frame_duration_ms;
        
        // Flush context to ensure texture is ready
        unsafe {
            context.Flush();
        }
        
        // Encode the texture - try multiple times for first frame
        let texture_ptr = texture.as_raw() as *mut c_void;
        let mut encode_result = encoder.encode(texture_ptr, pts);
        
        // For first frame, try encoding a few more times as encoder may need warmup
        if frame_num == 0 && encode_result.is_err() {
            log::warn!("First frame encode failed, retrying...");
            for retry in 1..=3 {
                std::thread::sleep(Duration::from_millis(10));
                encode_result = encoder.encode(texture_ptr, pts);
                if encode_result.is_ok() {
                    log::info!("First frame encode succeeded on retry {}", retry);
                    break;
                }
            }
        }
        
        match encode_result {
            Ok(frames) => {
                if frames.is_empty() {
                    log::warn!("Frame {} encoded but no data returned", frame_num);
                    // Continue to next frame
                } else {
                    for frame in frames.iter() {
                        // Write encoded frame data to file
                        if let Err(e) = output_file.write_all(&frame.data) {
                            log::error!("Failed to write frame data: {:?}", e);
                            return;
                        }
                    }
                    if frame_num % 30 == 0 || frame_num < 5 {
                        log::info!("Encoded frame {}/{} ({} bytes)", frame_num + 1, total_frames, 
                            frames.iter().map(|f| f.data.len()).sum::<usize>());
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to encode frame {}: error code {}", frame_num, e);
                if frame_num == 0 {
                    log::error!("First frame encoding failed. This may indicate:");
                    log::error!("  1. Texture format mismatch");
                    log::error!("  2. Encoder initialization issue");
                    log::error!("  3. AMF QueryOutput returned non-OK status");
                }
                return;
            }
        }
        
        // Small delay to simulate real-time encoding
        std::thread::sleep(Duration::from_millis(frame_duration_ms as u64));
    }
    
    let elapsed = start_time.elapsed();
    log::info!(
        "Encoding completed! Encoded {} frames in {:?}",
        total_frames,
        elapsed
    );
    log::info!("Output saved to output/output.h264");
}

#[cfg(windows)]
fn create_d3d11_device_and_texture(
    width: i32,
    height: i32,
) -> std::result::Result<(ID3D11Device, ID3D11Texture2D), Error> {
    unsafe {
        // Create DXGI Factory
        let factory: IDXGIFactory1 = CreateDXGIFactory1()?;
        
        // Get first adapter
        let adapter1 = match factory.EnumAdapters1(0) {
            Ok(a) => a,
            Err(_) => return Err(Error::from(E_FAIL)),
        };
        
        // Convert IDXGIAdapter1 to IDXGIAdapter
        let adapter: IDXGIAdapter = adapter1.cast()?;
        
        // Create D3D11 device
        let mut device: Option<ID3D11Device> = None;
        let mut device_context: Option<ID3D11DeviceContext> = None;
        let feature_levels = [D3D_FEATURE_LEVEL_11_0];
        let mut feature_level = D3D_FEATURE_LEVEL_11_0;
        
        D3D11CreateDevice(
            Some(&adapter),
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_VIDEO_SUPPORT | D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut feature_level),
            Some(&mut device_context),
        )?;
        
        let device = device.ok_or(Error::from(E_FAIL))?;
        
        // Create BGRA texture - use DEFAULT for AMF encoder (needs GPU memory)
        let texture_desc = D3D11_TEXTURE2D_DESC {
            Width: width as u32,
            Height: height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: Default::default(),
        };
        
        let mut texture: Option<ID3D11Texture2D> = None;
        device.CreateTexture2D(&texture_desc, None, Some(&mut texture))?;
        let texture = texture.ok_or(Error::from(E_FAIL))?;
        
        Ok((device, texture))
    }
}

#[cfg(windows)]
fn generate_test_texture(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    width: i32,
    height: i32,
    frame_num: i32,
) {
    unsafe {
        // Create a staging texture for CPU access
        let staging_desc = D3D11_TEXTURE2D_DESC {
            Width: width as u32,
            Height: height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            MiscFlags: Default::default(),
        };
        
        let mut staging_texture: Option<ID3D11Texture2D> = None;
        if let Err(_) = device.CreateTexture2D(&staging_desc, None, Some(&mut staging_texture)) {
            log::warn!("Failed to create staging texture");
            return;
        }
        let staging_texture = match staging_texture {
            Some(t) => t,
            None => {
                log::warn!("Failed to create staging texture");
                return;
            }
        };
        
        // Map staging texture for writing
        let mut mapped_resource = D3D11_MAPPED_SUBRESOURCE::default();
        let map_result = context.Map(&staging_texture, 0, D3D11_MAP_WRITE, 0, Some(&mut mapped_resource));
        if map_result.is_ok() {
            let row_pitch = mapped_resource.RowPitch as usize;
            let data = std::slice::from_raw_parts_mut(
                mapped_resource.pData as *mut u8,
                row_pitch * height as usize,
            );
            
            // Generate continuous color gradient pattern
            // Create a smooth gradient that changes over time
            let time = (frame_num as f32) * 0.02; // Slower change for smoother gradient
            for y in 0..height {
                for x in 0..width {
                    let offset = (y * row_pitch as i32 + x * 4) as usize;
                    if offset + 3 < data.len() {
                        // BGRA format
                        // Create smooth horizontal and vertical gradients
                        let x_ratio = x as f32 / width as f32;
                        let y_ratio = y as f32 / height as f32;
                        
                        // Horizontal gradient (red channel) - changes with time
                        let r = ((x_ratio * 255.0 + time * 50.0) % 255.0) as u8;
                        
                        // Vertical gradient (green channel) - changes with time
                        let g = ((y_ratio * 255.0 + time * 30.0) % 255.0) as u8;
                        
                        // Diagonal gradient (blue channel) - changes with time
                        let diagonal = (x_ratio + y_ratio) / 2.0;
                        let b = ((diagonal * 255.0 + time * 40.0) % 255.0) as u8;
                        
                        data[offset] = b;     // B
                        data[offset + 1] = g; // G
                        data[offset + 2] = r; // R
                        data[offset + 3] = 255; // A
                    }
                }
            }
            
            context.Unmap(&staging_texture, 0);
            
            // Copy from staging to default texture
            context.CopyResource(texture, &staging_texture);
            context.Flush();
        } else {
            log::warn!("Failed to map staging texture for writing");
        }
    }
}

#[cfg(not(windows))]
fn main() {
    println!("This example only works on Windows");
}
