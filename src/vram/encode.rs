use crate::{
    common::Driver::*,
    vram::{
        amf, inner::EncodeBackend, mfx, nv,
        DynamicContext, EncodeContext, FeatureContext,
    },
};
use log::trace;
use std::fmt::Display;

pub use crate::vram::inner::EncodeFrame;

pub struct Encoder {
    backend: Box<dyn EncodeBackend>,
    frames: Vec<EncodeFrame>,
    pub ctx: EncodeContext,
}

unsafe impl Send for Encoder {}
unsafe impl Sync for Encoder {}

impl Encoder {
    pub fn new(ctx: EncodeContext) -> Result<Self, ()> {
        if ctx.d.width % 2 == 1 || ctx.d.height % 2 == 1 {
            return Err(());
        }
        let device = ctx.d.device.unwrap_or(std::ptr::null_mut());
        let backend: Box<dyn EncodeBackend> = match ctx.f.driver {
            NV => nv::create_encode_backend(device, ctx.f.luid, ctx.f.data_format as i32,
                ctx.d.width, ctx.d.height, ctx.d.kbitrate, ctx.d.framerate, ctx.d.gop)?,
            AMF => amf::create_encode_backend(device, ctx.f.luid, ctx.f.data_format as i32,
                ctx.d.width, ctx.d.height, ctx.d.kbitrate, ctx.d.framerate, ctx.d.gop)?,
            MFX => mfx::create_encode_backend(device, ctx.f.luid, ctx.f.data_format as i32,
                ctx.d.width, ctx.d.height, ctx.d.kbitrate, ctx.d.framerate, ctx.d.gop)?,
        };
        Ok(Self {
            backend,
            frames: Vec::new(),
            ctx,
        })
    }

    pub fn encode(&mut self, tex: *mut std::ffi::c_void, ms: i64) -> Result<&mut Vec<EncodeFrame>, i32> {
        self.frames.clear();
        self.backend.encode(tex, ms, &mut self.frames)?;
        Ok(&mut self.frames)
    }

    pub fn set_bitrate(&mut self, kbs: i32) -> Result<(), i32> {
        self.backend.set_bitrate(kbs)
    }

    pub fn set_framerate(&mut self, framerate: i32) -> Result<(), i32> {
        self.backend.set_framerate(framerate)
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        self.backend.destroy();
        trace!("Encoder dropped");
    }
}

impl Display for EncodeFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "encode len:{}, key:{}", self.data.len(), self.key)
    }
}

pub fn available(d: DynamicContext) -> Vec<FeatureContext> {
    use log::debug;

    let mut natives: Vec<_> = vec![];
    natives.append(
        &mut nv::possible_support_encoders()
            .drain(..)
            .map(|n| (NV, n))
            .collect(),
    );
    natives.append(
        &mut amf::possible_support_encoders()
            .drain(..)
            .map(|n| (AMF, n))
            .collect(),
    );
    natives.append(
        &mut mfx::possible_support_encoders()
            .drain(..)
            .map(|n| (MFX, n))
            .collect(),
    );
    debug!(
        "编码器候选: {} 个 (driver+format) -> 将逐项 test，通过后可用；纹理与编码器须使用同一 D3D11 设备",
        natives.len()
    );
    let inputs: Vec<EncodeContext> = natives
        .drain(..)
        .map(|(driver, n)| EncodeContext {
            f: FeatureContext {
                driver: driver.clone(),
                vendor: driver, // Initially set vendor same as driver, will be updated by test results
                data_format: n.format,
                luid: 0,
            },
            d,
        })
        .collect();

    let mut outputs = Vec::<EncodeContext>::new();
    let mut exclude_luid_formats = Vec::<(i64, i32)>::new();

    for input in inputs {
        debug!(
            "Testing vram encoder: driver={:?}, format={:?}",
            input.f.driver, input.f.data_format
        );

        let test = match input.f.driver {
            NV => nv::encode_calls().test,
            AMF => amf::encode_calls().test,
            MFX => mfx::encode_calls().test,
        };

        let mut luids: Vec<i64> = vec![0; crate::vram::MAX_ADATERS];
        let mut vendors: Vec<i32> = vec![0; crate::vram::MAX_ADATERS];
        let mut desc_count: i32 = 0;

        let (excluded_luids, exclude_formats): (Vec<i64>, Vec<i32>) = exclude_luid_formats
            .iter()
            .map(|(luid, format)| (*luid, *format))
            .unzip();

        let result = unsafe {
            test(
                luids.as_mut_ptr(),
                vendors.as_mut_ptr(),
                luids.len() as _,
                &mut desc_count,
                input.f.data_format as i32,
                input.d.width,
                input.d.height,
                input.d.kbitrate,
                input.d.framerate,
                input.d.gop,
                excluded_luids.as_ptr(),
                exclude_formats.as_ptr(),
                exclude_luid_formats.len() as i32,
            )
        };

        if result == 0 {
            if desc_count as usize <= luids.len() {
                debug!(
                    "vram encoder test passed: driver={:?}, adapters={}",
                    input.f.driver, desc_count
                );
                for i in 0..desc_count as usize {
                    let mut input = input.clone();
                    input.f.luid = luids[i];
                    input.f.vendor = match vendors[i] {
                        0 => NV,
                        1 => AMF,
                        2 => MFX,
                        _ => {
                            log::error!(
                                "Unexpected vendor value encountered: {}. Skipping.",
                                vendors[i]
                            );
                            continue;
                        },
                    };
                    exclude_luid_formats.push((luids[i], input.f.data_format as i32));
                    outputs.push(input);
                }
            }
        } else {
            debug!(
                "vram encoder test failed: driver={:?}, error={}",
                input.f.driver, result
            );
        }
    }

    let result: Vec<_> = outputs.drain(..).map(|e| e.f).collect();
    result
}
