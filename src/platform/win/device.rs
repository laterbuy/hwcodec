//! D3D11 设备管理核心实现

use crate::common;
use crate::platform::win::error::{Result, WinPlatformError};
use windows::core::Interface;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Dxgi::Common::*;

/// D3D11 原生设备，提供视频处理和格式转换功能
pub struct NativeDevice {
    #[allow(dead_code)]
    factory: IDXGIFactory1,
    #[allow(dead_code)]
    adapter: IDXGIAdapter,
    adapter1: IDXGIAdapter1,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    query: ID3D11Query,
    video_device: ID3D11VideoDevice,
    video_context: ID3D11VideoContext,
    video_context1: ID3D11VideoContext1,
    video_processor_enumerator: Option<ID3D11VideoProcessorEnumerator>,
    video_processor: Option<ID3D11VideoProcessor>,
    last_content_desc: Option<D3D11_VIDEO_PROCESSOR_CONTENT_DESC>,
    
    // 纹理池
    textures: Vec<Option<ID3D11Texture2D>>,
    current_index: usize,
    pool_size: usize,
    
    // NV12 到 BGRA 转换相关
    rtv: Option<ID3D11RenderTargetView>,
    srv: [Option<ID3D11ShaderResourceView>; 2],
    vertex_shader: Option<ID3D11VertexShader>,
    pixel_shader: Option<ID3D11PixelShader>,
    sampler_linear: Option<ID3D11SamplerState>,
    nv12_srv_texture: Option<ID3D11Texture2D>,
    last_nv12_to_bgra_width: u32,
    last_nv12_to_bgra_height: u32,
}

impl NativeDevice {
    /// 创建新的 NativeDevice
    /// 
    /// # 参数
    /// - `luid`: 适配器 LUID，如果为 0 则使用默认适配器
    /// - `device`: 可选的现有 D3D11 设备，如果提供则使用该设备
    /// - `pool_size`: 纹理池大小
    pub fn new(luid: i64, device: Option<*mut ID3D11Device>, pool_size: usize) -> Result<Self> {
        let (factory, adapter, adapter1, device, context) = if let Some(device_ptr) = device {
            Self::init_from_device(unsafe { Interface::from_raw(device_ptr as *mut std::ffi::c_void) })?
        } else {
            Self::init_from_luid(luid)?
        };

        let mut native_device = Self {
            factory,
            adapter,
            adapter1,
            device,
            context,
            #[allow(invalid_value)]
            query: unsafe {
                // 这些字段会在后续立即初始化（在 new 函数中），不会实际使用零值
                std::mem::MaybeUninit::<ID3D11Query>::zeroed().assume_init()
            },
            #[allow(invalid_value)]
            video_device: unsafe {
                std::mem::MaybeUninit::<ID3D11VideoDevice>::zeroed().assume_init()
            },
            #[allow(invalid_value)]
            video_context: unsafe {
                std::mem::MaybeUninit::<ID3D11VideoContext>::zeroed().assume_init()
            },
            #[allow(invalid_value)]
            video_context1: unsafe {
                std::mem::MaybeUninit::<ID3D11VideoContext1>::zeroed().assume_init()
            },
            video_processor_enumerator: None,
            video_processor: None,
            last_content_desc: None,
            textures: Vec::new(),
            current_index: 0,
            pool_size,
            rtv: None,
            srv: [None, None],
            vertex_shader: None,
            pixel_shader: None,
            sampler_linear: None,
            nv12_srv_texture: None,
            last_nv12_to_bgra_width: 0,
            last_nv12_to_bgra_height: 0,
        };

        native_device.set_multithread_protected()?;
        native_device.init_query()?;
        native_device.init_video_device()?;
        
        // 初始化纹理池（使用空 Vec，稍后填充）
        native_device.textures = Vec::with_capacity(pool_size);

        Ok(native_device)
    }

    /// 从 LUID 初始化设备
    fn init_from_luid(luid: i64) -> Result<(
        IDXGIFactory1,
        IDXGIAdapter,
        IDXGIAdapter1,
        ID3D11Device,
        ID3D11DeviceContext,
    )> {
        let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };

        let mut adapter1: Option<IDXGIAdapter1> = None;
        let mut adapter_index = 0;

        // 枚举适配器，查找匹配的 LUID
        loop {
            let tmp_adapter: IDXGIAdapter1 = unsafe {
                match factory.EnumAdapters1(adapter_index) {
                    Ok(a) => a,
                    Err(_) => break, // 枚举结束
                }
            };

            let desc = unsafe { tmp_adapter.GetDesc1()? };
            let adapter_luid = ((desc.AdapterLuid.HighPart as i64) << 32)
                | desc.AdapterLuid.LowPart as i64;

            if adapter_luid == luid {
                adapter1 = Some(tmp_adapter);
                break;
            }

            adapter_index += 1;
        }

        let adapter1 = adapter1.ok_or(WinPlatformError::AdapterNotFound)?;
        let adapter: IDXGIAdapter = windows::core::Interface::cast(&adapter1)?;

        // 创建 D3D11 设备
        let feature_levels = [D3D_FEATURE_LEVEL_11_0];
        let create_flags = D3D11_CREATE_DEVICE_FLAG(
            D3D11_CREATE_DEVICE_VIDEO_SUPPORT.0 | D3D11_CREATE_DEVICE_BGRA_SUPPORT.0,
        );

        let mut device = None;
        let mut context = None;
        let mut feature_level = D3D_FEATURE_LEVEL(0);

        unsafe {
            // D3D11CreateDevice需要IDXGIAdapter，需要转换
            let adapter_base: IDXGIAdapter = Interface::cast(&adapter1)?;
            D3D11CreateDevice(
                &adapter_base,
                D3D_DRIVER_TYPE_UNKNOWN,
                windows::Win32::Foundation::HMODULE::default(),
                create_flags,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut device),
                Some(&mut feature_level),
                Some(&mut context),
            )?;
        }

        let device = device.unwrap();
        let context = context.unwrap();

        if feature_level != D3D_FEATURE_LEVEL_11_0 {
            return Err(WinPlatformError::UnsupportedFeatureLevel(feature_level));
        }

        Ok((factory, adapter, adapter1, device, context))
    }

    /// 从现有设备初始化
    fn init_from_device(
        device: ID3D11Device,
    ) -> Result<(
        IDXGIFactory1,
        IDXGIAdapter,
        IDXGIAdapter1,
        ID3D11Device,
        ID3D11DeviceContext,
    )> {
        let context = unsafe { device.GetImmediateContext()? };

        let dxgi_device: IDXGIDevice = Interface::cast(&device)?;
        let adapter: IDXGIAdapter = unsafe { dxgi_device.GetAdapter()? };
        let adapter1: IDXGIAdapter1 = Interface::cast(&adapter)?;
        let factory: IDXGIFactory1 = unsafe { adapter1.GetParent()? };

        Ok((factory, adapter, adapter1, device, context))
    }

    /// 设置多线程保护
    fn set_multithread_protected(&self) -> Result<()> {
        unsafe {
            let multithread: windows::Win32::Graphics::Direct3D10::ID3D10Multithread =
                Interface::cast(&self.context)?;
            let _ = multithread.SetMultithreadProtected(true);
            Ok(())
        }
    }

    /// 初始化查询对象
    fn init_query(&mut self) -> Result<()> {
        let query_desc = D3D11_QUERY_DESC {
            Query: D3D11_QUERY_EVENT,
            MiscFlags: Default::default(),
        };

        unsafe {
            let mut query = None;
            self.device.CreateQuery(&query_desc, Some(&mut query))?;
            self.query = query.unwrap();
        }

        Ok(())
    }

    /// 初始化视频设备
    fn init_video_device(&mut self) -> Result<()> {
        self.video_device = Interface::cast(&self.device)?;
        self.video_context = Interface::cast(&self.context)?;
        self.video_context1 = Interface::cast(&self.video_context)?;
        Ok(())
    }

    /// 获取设备
    pub fn device(&self) -> &ID3D11Device {
        &self.device
    }

    /// 获取上下文
    pub fn context(&self) -> &ID3D11DeviceContext {
        &self.context
    }

    /// 获取视频设备
    pub fn video_device(&self) -> &ID3D11VideoDevice {
        &self.video_device
    }

    /// 获取视频上下文
    pub fn video_context(&self) -> &ID3D11VideoContext {
        &self.video_context
    }

    /// 获取视频上下文1
    pub fn video_context1(&self) -> &ID3D11VideoContext1 {
        &self.video_context1
    }

    /// 获取厂商
    pub fn get_vendor(&self) -> common::AdapterVendor {
        let desc = unsafe { self.adapter1.GetDesc1().unwrap() };
        match desc.VendorId {
            0x10DE => common::AdapterVendor::ADAPTER_VENDOR_NVIDIA,
            0x1002 => common::AdapterVendor::ADAPTER_VENDOR_AMD,
            0x8086 => common::AdapterVendor::ADAPTER_VENDOR_INTEL,
            _ => common::AdapterVendor::ADAPTER_VENDOR_UNKNOWN,
        }
    }

    /// 确保存在指定尺寸的共享纹理
    pub fn ensure_texture(&mut self, width: u32, height: u32) -> Result<()> {
        // 检查现有纹理是否满足要求
        if let Some(Some(existing)) = self.textures.first() {
            let desc = unsafe {
                let mut desc = std::mem::zeroed();
                existing.GetDesc(&mut desc);
                desc
            };

                if desc.Width == width
                    && desc.Height == height
                    && desc.Format == DXGI_FORMAT_B8G8R8A8_UNORM
                    && (desc.MiscFlags as u32 & D3D11_RESOURCE_MISC_SHARED.0 as u32) != 0
                    && desc.Usage == D3D11_USAGE_DEFAULT
            {
                return Ok(()); // 现有纹理满足要求
            }
        }

        // 创建新纹理
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
            CPUAccessFlags: Default::default(),
            MiscFlags: D3D11_RESOURCE_MISC_SHARED.0 as u32,
        };

        self.textures.clear();
        for _i in 0..self.pool_size {
            unsafe {
                let mut texture = None;
                self.device.CreateTexture2D(&desc, None, Some(&mut texture))?;
                self.textures.push(Some(texture.unwrap()));
            }
        }

        Ok(())
    }

    /// 设置当前使用的纹理
    pub fn set_texture(&mut self, texture: ID3D11Texture2D) {
        while self.textures.len() <= self.current_index {
            self.textures.push(None);
        }
        self.textures[self.current_index] = Some(texture);
    }

    /// 获取共享句柄
    pub fn get_shared_handle(&self) -> Result<HANDLE> {
        if self.textures.len() <= self.current_index {
            return Err(WinPlatformError::InvalidParameter(
                "No texture available".to_string(),
            ));
        }

        let texture = self.textures[self.current_index].as_ref().ok_or_else(|| {
            WinPlatformError::InvalidParameter("No texture at current index".to_string())
        })?;
        unsafe {
            let resource: IDXGIResource = texture.cast()?;
            let handle = resource.GetSharedHandle()?;
            Ok(handle)
        }
    }

    /// 获取当前纹理
    pub fn get_current_texture(&self) -> Option<&ID3D11Texture2D> {
        self.textures.get(self.current_index).and_then(|t| t.as_ref())
    }

    /// 切换到下一个纹理（循环）
    pub fn next(&mut self) -> usize {
        self.current_index = (self.current_index + 1) % self.pool_size;
        self.current_index
    }

    /// 开始查询
    pub fn begin_query(&self) {
        unsafe {
            self.context.Begin(&self.query);
        }
    }

    /// 结束查询
    pub fn end_query(&self) {
        unsafe {
            self.context.End(&self.query);
        }
    }

    /// 等待查询完成
    pub fn query(&self) -> Result<bool> {
        let mut result = false;
        let mut attempts = 0;

        loop {
            unsafe {
                let hr = self.context.GetData(
                    &self.query,
                    Some(&mut result as *mut _ as *mut std::ffi::c_void),
                    std::mem::size_of::<bool>() as u32,
                    0,
                );

                if hr.is_ok() && result {
                    return Ok(true);
                }

                attempts += 1;
                if attempts > 100 {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                if attempts > 1000 {
                    break;
                }
            }
        }

        Ok(result)
    }

    /// 视频处理（使用 D3D11 Video Processor）
    /// 
    /// # 参数
    /// - `input`: 输入纹理
    /// - `output`: 输出纹理
    /// - `width`: 处理宽度
    /// - `height`: 处理高度
    /// - `content_desc`: 视频内容描述
    /// - `color_space_in`: 输入颜色空间
    /// - `color_space_out`: 输出颜色空间
    /// - `array_slice`: 数组切片索引
    pub fn process(
        &mut self,
        input: &ID3D11Texture2D,
        output: &ID3D11Texture2D,
        width: u32,
        height: u32,
        content_desc: D3D11_VIDEO_PROCESSOR_CONTENT_DESC,
        color_space_in: DXGI_COLOR_SPACE_TYPE,
        color_space_out: DXGI_COLOR_SPACE_TYPE,
        array_slice: u32,
    ) -> Result<()> {
        // 检查内容描述是否变化，如果变化则重新创建视频处理器
        let need_recreate = self
            .last_content_desc
            .as_ref()
            .map(|last| {
                // 比较结构体内容
                last.InputFrameFormat != content_desc.InputFrameFormat
                    || last.InputWidth != content_desc.InputWidth
                    || last.InputHeight != content_desc.InputHeight
                    || last.OutputWidth != content_desc.OutputWidth
                    || last.OutputHeight != content_desc.OutputHeight
            })
            .unwrap_or(true);

        if need_recreate {
            self.video_processor_enumerator = None;
            self.video_processor = None;
            self.last_content_desc = Some(content_desc);
        }

        // 创建视频处理器枚举器（如果不存在）
        if self.video_processor_enumerator.is_none() {
            unsafe {
                let enumerator = self.video_device.CreateVideoProcessorEnumerator(&content_desc)?;
                self.video_processor_enumerator = Some(enumerator);
            }
        }

        // 创建视频处理器（如果不存在）
        if self.video_processor.is_none() {
            let enumerator = self.video_processor_enumerator.as_ref().unwrap();
            unsafe {
                let processor = self.video_device.CreateVideoProcessor(enumerator, 0)?;
                self.video_processor = Some(processor);

                // 配置视频处理器
                let processor = self.video_processor.as_ref().unwrap();
                self.video_context.VideoProcessorSetStreamAutoProcessingMode(
                    processor,
                    0,
                    false,
                );
                self.video_context.VideoProcessorSetStreamFrameFormat(
                    processor,
                    0,
                    D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
                );
            }
        }

        let processor = self.video_processor.as_ref().unwrap();

        // 设置颜色空间
        unsafe {
            self.video_context1.VideoProcessorSetStreamColorSpace1(
                processor,
                0,
                color_space_in,
            );
            self.video_context1.VideoProcessorSetOutputColorSpace1(
                processor,
                color_space_out,
            );
        }

        // 设置源和目标矩形
        let rect = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };

        unsafe {
            self.video_context.VideoProcessorSetStreamSourceRect(
                processor,
                0,
                true,
                Some(&rect),
            );
            self.video_context1.VideoProcessorSetStreamDestRect(
                processor,
                0,
                true,
                Some(&rect),
            );
        }

        // 创建输入视图
        let mut input_view_desc = unsafe { std::mem::zeroed::<D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC>() };
        input_view_desc.FourCC = 0;
        input_view_desc.ViewDimension = D3D11_VPIV_DIMENSION_TEXTURE2D;
        input_view_desc.Anonymous.Texture2D.MipSlice = 0;
        input_view_desc.Anonymous.Texture2D.ArraySlice = array_slice;

        let enumerator = self.video_processor_enumerator.as_ref().unwrap();
        let input_view = unsafe {
            let mut view = None;
            self.video_device.CreateVideoProcessorInputView(
                input,
                enumerator,
                &input_view_desc,
                Some(&mut view),
            )?;
            view.unwrap()
        };

        // 创建输出视图
        let mut output_view_desc = unsafe { std::mem::zeroed::<D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC>() };
        output_view_desc.ViewDimension = D3D11_VPOV_DIMENSION_TEXTURE2D;
        output_view_desc.Anonymous.Texture2D.MipSlice = 0;

        let output_view = unsafe {
            let mut view = None;
            self.video_device.CreateVideoProcessorOutputView(
                output,
                enumerator,
                &output_view_desc,
                Some(&mut view),
            )?;
            view.unwrap()
        };

        // 执行视频处理
        // 注意：pInputSurface需要保持input_view的生命周期
        let stream_data = unsafe {
            let mut data = std::mem::zeroed::<D3D11_VIDEO_PROCESSOR_STREAM>();
            data.Enable = true.into();
            data.OutputIndex = 0;
            data.InputFrameOrField = 0;
            data.PastFrames = 0;
            data.FutureFrames = 0;
            data.pInputSurface = std::mem::ManuallyDrop::new(Some(input_view));
            data.ppPastSurfaces = std::ptr::null_mut();
            data.ppFutureSurfaces = std::ptr::null_mut();
            data.ppPastSurfacesRight = std::ptr::null_mut();
            data.ppFutureSurfacesRight = std::ptr::null_mut();
            data
        };

        unsafe {
            self.video_context.VideoProcessorBlt(
                processor,
                &output_view,
                0,
                &mut [stream_data],
            )?;
        }

        Ok(())
    }

    /// 将 BGRA 纹理转换为 NV12 格式
    pub fn bgra_to_nv12(
        &mut self,
        bgra_texture: &ID3D11Texture2D,
        nv12_texture: &ID3D11Texture2D,
        width: u32,
        height: u32,
        color_space_in: DXGI_COLOR_SPACE_TYPE,
        color_space_out: DXGI_COLOR_SPACE_TYPE,
    ) -> Result<()> {
        // 检查纹理尺寸
        let bgra_desc = unsafe {
            let mut desc = std::mem::zeroed();
            bgra_texture.GetDesc(&mut desc);
            desc
        };

        let nv12_desc = unsafe {
            let mut desc = std::mem::zeroed();
            nv12_texture.GetDesc(&mut desc);
            desc
        };

        if bgra_desc.Width < width || bgra_desc.Height < height {
            return Err(WinPlatformError::InvalidParameter(format!(
                "BGRA texture size {}x{} is smaller than {}x{}",
                bgra_desc.Width, bgra_desc.Height, width, height
            )));
        }

        if nv12_desc.Width < width || nv12_desc.Height < height {
            return Err(WinPlatformError::InvalidParameter(format!(
                "NV12 texture size {}x{} is smaller than {}x{}",
                nv12_desc.Width, nv12_desc.Height, width, height
            )));
        }

        // 创建内容描述
        let content_desc = D3D11_VIDEO_PROCESSOR_CONTENT_DESC {
            InputFrameFormat: D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            InputFrameRate: DXGI_RATIONAL {
                Numerator: 30,
                Denominator: 1,
            },
            InputWidth: width,
            InputHeight: height,
            OutputFrameRate: DXGI_RATIONAL {
                Numerator: 30,
                Denominator: 1,
            },
            OutputWidth: width,
            OutputHeight: height,
            Usage: D3D11_VIDEO_USAGE_PLAYBACK_NORMAL,
        };

        // 使用视频处理器进行转换
        self.process(
            bgra_texture,
            nv12_texture,
            width,
            height,
            content_desc,
            color_space_in,
            color_space_out,
            0,
        )
    }

    /// 将 NV12 纹理转换为 BGRA 格式（使用 GPU 着色器）
    pub fn nv12_to_bgra(
        &mut self,
        nv12_texture: &ID3D11Texture2D,
        bgra_texture: &ID3D11Texture2D,
        width: u32,
        height: u32,
        nv12_array_index: u32,
    ) -> Result<()> {
        // 如果尺寸变化，重新设置着色器状态
        if width != self.last_nv12_to_bgra_width
            || height != self.last_nv12_to_bgra_height
        {
            self.nv12_to_bgra_set_srv(nv12_texture, width, height)?;
            self.nv12_to_bgra_set_viewport(width, height)?;
            self.nv12_to_bgra_set_sample()?;
            self.nv12_to_bgra_set_shader()?;
            self.nv12_to_bgra_set_vertex_buffer()?;
        }

        self.last_nv12_to_bgra_width = width;
        self.last_nv12_to_bgra_height = height;

        self.nv12_to_bgra_set_rtv(bgra_texture, width, height)?;

        // 复制纹理数据
        let src_box = windows::Win32::Graphics::Direct3D11::D3D11_BOX {
            left: 0,
            top: 0,
            front: 0,
            right: width,
            bottom: height,
            back: 1,
        };

        unsafe {
            self.context.CopySubresourceRegion(
                self.nv12_srv_texture.as_ref().unwrap(),
                0,
                0,
                0,
                0,
                nv12_texture,
                nv12_array_index,
                Some(&src_box),
            );
        }

        self.nv12_to_bgra_draw()?;

        Ok(())
    }

    /// 设置着色器资源视图（SRV）
    fn nv12_to_bgra_set_srv(
        &mut self,
        nv12_texture: &ID3D11Texture2D,
        width: u32,
        height: u32,
    ) -> Result<()> {
        self.srv[0] = None;
        self.srv[1] = None;

        // 获取纹理描述（用于验证，但当前不需要）
        let _tex_desc = unsafe {
            let mut desc = std::mem::zeroed();
            nv12_texture.GetDesc(&mut desc);
            desc
        };

        // 创建 SRV 纹理
        let srv_tex_desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_NV12,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: Default::default(),
            MiscFlags: Default::default(),
        };

        unsafe {
            let mut texture = None;
            self.device.CreateTexture2D(&srv_tex_desc, None, Some(&mut texture))?;
            self.nv12_srv_texture = Some(texture.unwrap());
        }

        let srv_texture = self.nv12_srv_texture.as_ref().unwrap();

        // 创建 Y 平面 SRV (R8_UNORM)
        let srv_desc_y = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_R8_UNORM,
            ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
            Anonymous: windows::Win32::Graphics::Direct3D11::D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_SRV {
                    MostDetailedMip: 0,
                    MipLevels: 1,
                },
            },
        };

        unsafe {
            let mut srv = None;
            self.device.CreateShaderResourceView(
                srv_texture,
                Some(&srv_desc_y),
                Some(&mut srv),
            )?;
            self.srv[0] = Some(srv.unwrap());
        }

        // 创建 UV 平面 SRV (R8G8_UNORM)
        let srv_desc_uv = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_R8G8_UNORM,
            ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
            Anonymous: windows::Win32::Graphics::Direct3D11::D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_SRV {
                    MostDetailedMip: 0,
                    MipLevels: 1,
                },
            },
        };

        unsafe {
            let mut srv = None;
            self.device.CreateShaderResourceView(
                srv_texture,
                Some(&srv_desc_uv),
                Some(&mut srv),
            )?;
            self.srv[1] = Some(srv.unwrap());
        }

        // 设置 SRV
        unsafe {
            // 获取原始指针并创建新的接口引用（不增加引用计数）
            let srv0_ptr = self.srv[0].as_ref().unwrap().as_raw() as *mut _;
            let srv1_ptr = self.srv[1].as_ref().unwrap().as_raw() as *mut _;
            let srv0 = Interface::from_raw(srv0_ptr);
            let srv1 = Interface::from_raw(srv1_ptr);
            let srv_ptrs = [Some(srv0), Some(srv1)];
            self.context.PSSetShaderResources(0, Some(&srv_ptrs));
            // 使用 ManuallyDrop 避免释放（这些是临时引用）
            std::mem::forget(srv_ptrs);
        }

        Ok(())
    }

    /// 设置渲染目标视图（RTV）
    fn nv12_to_bgra_set_rtv(
        &mut self,
        bgra_texture: &ID3D11Texture2D,
        _width: u32,
        _height: u32,
    ) -> Result<()> {
        self.rtv = None;

        let rt_desc = D3D11_RENDER_TARGET_VIEW_DESC {
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
            Anonymous: windows::Win32::Graphics::Direct3D11::D3D11_RENDER_TARGET_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_RTV { MipSlice: 0 },
            },
        };

        unsafe {
            let mut rtv = None;
            self.device.CreateRenderTargetView(
                bgra_texture,
                Some(&rt_desc),
                Some(&mut rtv),
            )?;
            self.rtv = Some(rtv.unwrap());

            // 清除渲染目标
            let clear_color = [0.0f32, 0.0f32, 0.0f32, 0.0f32];
            self.context.ClearRenderTargetView(self.rtv.as_ref().unwrap(), &clear_color);

            // 设置渲染目标
            let rtv_ptr = self.rtv.as_ref().unwrap().as_raw() as *mut _;
            let rtv = Interface::from_raw(rtv_ptr);
            let rtv_ptrs = [Some(rtv)];
            self.context.OMSetRenderTargets(Some(&rtv_ptrs), None);
            std::mem::forget(rtv_ptrs);
        }

        Ok(())
    }

    /// 设置视口
    fn nv12_to_bgra_set_viewport(&self, width: u32, height: u32) -> Result<()> {
        let viewport = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: width as f32,
            Height: height as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };

        unsafe {
            self.context.RSSetViewports(Some(&[viewport]));
        }

        Ok(())
    }

    /// 设置采样器
    fn nv12_to_bgra_set_sample(&mut self) -> Result<()> {
        self.sampler_linear = None;

        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            MipLODBias: 0.0,
            MaxAnisotropy: 1,
            ComparisonFunc: D3D11_COMPARISON_NEVER,
            BorderColor: [0.0f32; 4],
            MinLOD: 0.0,
            MaxLOD: f32::MAX,
        };

        unsafe {
            let mut sampler = None;
            self.device.CreateSamplerState(&sampler_desc, Some(&mut sampler))?;
            self.sampler_linear = Some(sampler.unwrap());

            let sampler_ptr = self.sampler_linear.as_ref().unwrap().as_raw() as *mut _;
            let sampler = Interface::from_raw(sampler_ptr);
            let sampler_ptrs = [Some(sampler)];
            self.context.PSSetSamplers(0, Some(&sampler_ptrs));
            std::mem::forget(sampler_ptrs);
        }

        Ok(())
    }

    /// 设置着色器
    fn nv12_to_bgra_set_shader(&mut self) -> Result<()> {
        use crate::platform::win::shader;

        self.vertex_shader = Some(shader::create_vertex_shader(&self.device)?);
        self.pixel_shader = Some(shader::create_pixel_shader(&self.device)?);

        let input_layout = shader::create_input_layout(&self.device)?;

        unsafe {
            self.context.IASetInputLayout(Some(&input_layout));
            self.context.VSSetShader(
                Some(self.vertex_shader.as_ref().unwrap()),
                None,
            );
            self.context.PSSetShader(
                Some(self.pixel_shader.as_ref().unwrap()),
                None,
            );
        }

        Ok(())
    }

    /// 设置顶点缓冲区
    fn nv12_to_bgra_set_vertex_buffer(&self) -> Result<()> {
        #[repr(C)]
        struct Vertex {
            pos: [f32; 3],
            tex: [f32; 2],
        }

        const NUM_VERTICES: usize = 6;
        let vertices = [
            Vertex {
                pos: [-1.0f32, -1.0f32, 0.0f32],
                tex: [0.0f32, 1.0f32],
            },
            Vertex {
                pos: [-1.0f32, 1.0f32, 0.0f32],
                tex: [0.0f32, 0.0f32],
            },
            Vertex {
                pos: [1.0f32, -1.0f32, 0.0f32],
                tex: [1.0f32, 1.0f32],
            },
            Vertex {
                pos: [1.0f32, -1.0f32, 0.0f32],
                tex: [1.0f32, 1.0f32],
            },
            Vertex {
                pos: [-1.0f32, 1.0f32, 0.0f32],
                tex: [0.0f32, 0.0f32],
            },
            Vertex {
                pos: [1.0f32, 1.0f32, 0.0f32],
                tex: [1.0f32, 0.0f32],
            },
        ];

        let buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: (std::mem::size_of::<Vertex>() * NUM_VERTICES) as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            CPUAccessFlags: Default::default(),
            MiscFlags: Default::default(),
            StructureByteStride: 0,
        };

        let subresource_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: vertices.as_ptr() as *const _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        unsafe {
            let mut buffer = None;
            self.device.CreateBuffer(
                &buffer_desc,
                Some(&subresource_data),
                Some(&mut buffer),
            )?;
            let buffer = buffer.unwrap();

            let stride = std::mem::size_of::<Vertex>() as u32;
            let offset = 0u32;
            let buffers = [Some(buffer)];
            let strides = [stride];
            let offsets = [offset];
            self.context.IASetVertexBuffers(0, 1, Some(buffers.as_ptr()), Some(strides.as_ptr()), Some(offsets.as_ptr()));

            // 设置混合状态
            let blend_factor = [0.0f32; 4];
            self.context.OMSetBlendState(None, Some(&blend_factor), 0xffffffff);

            // 设置图元拓扑
            self.context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        }

        Ok(())
    }

    /// 执行绘制
    fn nv12_to_bgra_draw(&self) -> Result<()> {
        unsafe {
            self.context.Draw(6, 0);
            self.context.Flush();
        }
        Ok(())
    }

    /// 检查是否支持硬件解码
    pub fn support_decode(&self, format: common::DataFormat) -> Result<bool> {
        let guid = match format {
            common::DataFormat::H264 => D3D11_DECODER_PROFILE_H264_VLD_NOFGT,
            common::DataFormat::H265 => D3D11_DECODER_PROFILE_HEVC_VLD_MAIN,
            _ => return Ok(false),
        };

        let supported = unsafe {
            self.video_device.CheckVideoDecoderFormat(
                &guid,
                DXGI_FORMAT_NV12,
            ).is_ok()
        };

        if !supported {
            return Ok(false);
        }

        // 检查是否为混合解码
        let desc = unsafe { self.adapter1.GetDesc1()? };
        let is_hybrid = self.is_format_hybrid_decoded_by_hardware(
            format,
            desc.VendorId,
            desc.DeviceId,
        );

        Ok(!is_hybrid)
    }

    /// 检查特定 GPU 是否使用混合解码
    fn is_format_hybrid_decoded_by_hardware(
        &self,
        format: common::DataFormat,
        vendor_id: u32,
        device_id: u32,
    ) -> bool {
        if vendor_id == common::AdapterVendor::ADAPTER_VENDOR_INTEL as u32 {
            // Intel GPU 混合解码检测
            // 参考：https://github.com/moonlight-stream/moonlight-qt
            match device_id & 0xFF00 {
                0x0400 | 0x0A00 | 0x0D00 => {
                    // Haswell
                    return format == common::DataFormat::H265;
                }
                0x1600 => {
                    // Broadwell
                    return format == common::DataFormat::H265;
                }
                0x2200 => {
                    // Cherry Trail and Braswell
                    return format == common::DataFormat::H265;
                }
                _ => {}
            }
        } else if vendor_id == common::AdapterVendor::ADAPTER_VENDOR_NVIDIA as u32 {
            // NVIDIA GPU 混合解码检测（Feature Set E）
            // 参考：https://en.wikipedia.org/wiki/Nvidia_PureVideo
            if (device_id >= 0x1180 && device_id <= 0x11BF) // GK104
                || (device_id >= 0x11C0 && device_id <= 0x11FF) // GK106
                || (device_id >= 0x0FC0 && device_id <= 0x0FFF) // GK107
                || (device_id >= 0x1000 && device_id <= 0x103F) // GK110/GK110B
                || (device_id >= 0x1280 && device_id <= 0x12BF) // GK208
                || (device_id >= 0x1340 && device_id <= 0x137F) // GM108
                || (device_id >= 0x1380 && device_id <= 0x13BF) // GM107
                || (device_id >= 0x13C0 && device_id <= 0x13FF) // GM204
                || (device_id >= 0x1617 && device_id <= 0x161A) // GM204
                || (device_id == 0x1667) // GM204
                || (device_id >= 0x17C0 && device_id <= 0x17FF) // GM200
            {
                return format == common::DataFormat::H265;
            }
        }

        false
    }
}

// 实现 Send 和 Sync，因为 D3D11 上下文在多线程保护模式下是线程安全的
unsafe impl Send for NativeDevice {}
unsafe impl Sync for NativeDevice {}
