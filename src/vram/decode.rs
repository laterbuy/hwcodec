use crate::{
    common::{DataFormat::*, Driver::*},
    vram::{amf, inner::DecodeBackend, mfx, nv, DecodeContext},
};
use log::trace;

pub use crate::vram::inner::DecodeFrame;

pub struct Decoder {
    backend: Box<dyn DecodeBackend>,
    frames: Vec<DecodeFrame>,
    pub ctx: DecodeContext,
}

unsafe impl Send for Decoder {}
unsafe impl Sync for Decoder {}

impl Decoder {
    pub fn new(ctx: DecodeContext) -> Result<Self, ()> {
        let device = ctx.device.unwrap_or(std::ptr::null_mut());
        let backend: Box<dyn DecodeBackend> = match ctx.driver {
            NV => nv::create_decode_backend(device, ctx.luid, ctx.data_format as i32)?,
            AMF => amf::create_decode_backend(device, ctx.luid, ctx.data_format as i32)?,
            MFX => mfx::create_decode_backend(device, ctx.luid, ctx.data_format as i32)?,
        };
        Ok(Self {
            backend,
            frames: Vec::new(),
            ctx,
        })
    }

    pub fn decode(&mut self, packet: &[u8]) -> Result<&mut Vec<DecodeFrame>, i32> {
        self.frames.clear();
        self.backend.decode(packet, &mut self.frames)?;
        Ok(&mut self.frames)
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        self.backend.destroy();
        trace!("Decoder dropped");
    }
}

pub fn available() -> Vec<DecodeContext> {
    use log::debug;

    let mut codecs: Vec<_> = vec![];
    codecs.append(
        &mut nv::possible_support_decoders()
            .drain(..)
            .map(|n| (NV, n))
            .collect(),
    );
    codecs.append(
        &mut amf::possible_support_decoders()
            .drain(..)
            .map(|n| (AMF, n))
            .collect(),
    );
    codecs.append(
        &mut mfx::possible_support_decoders()
            .drain(..)
            .map(|n| (MFX, n))
            .collect(),
    );

    let inputs: Vec<DecodeContext> = codecs
        .drain(..)
        .map(|(driver, n)| DecodeContext {
            device: None,
            driver: driver.clone(),
            vendor: driver, // Initially set vendor same as driver, will be updated by test results
            data_format: n.data_format,
            luid: 0,
        })
        .collect();

    let mut outputs = Vec::<DecodeContext>::new();
    let mut exclude_luid_formats = Vec::<(i64, i32)>::new();
    let buf264 = &crate::common::DATA_H264_720P[..];
    let buf265 = &crate::common::DATA_H265_720P[..];

    for input in inputs {
        debug!(
            "Testing vram decoder: driver={:?}, format={:?}",
            input.driver, input.data_format
        );

        let test = match input.driver {
            NV => nv::decode_calls().test,
            AMF => amf::decode_calls().test,
            MFX => mfx::decode_calls().test,
        };

        let mut luids: Vec<i64> = vec![0; crate::vram::MAX_ADATERS];
        let mut vendors: Vec<i32> = vec![0; crate::vram::MAX_ADATERS];
        let mut desc_count: i32 = 0;

        let data = match input.data_format {
            H264 => buf264,
            H265 => buf265,
            _ => {
                debug!("Unsupported data format: {:?}, skipping", input.data_format);
                continue;
            }
        };

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
                input.data_format as i32,
                data.as_ptr() as *mut u8,
                data.len() as _,
                excluded_luids.as_ptr(),
                exclude_formats.as_ptr(),
                exclude_luid_formats.len() as i32,
            )
        };

        if result == 0 {
            if desc_count as usize <= luids.len() {
                debug!(
                    "vram decoder test passed: driver={:?}, adapters={}",
                    input.driver, desc_count
                );
                for i in 0..desc_count as usize {
                    let mut input = input.clone();
                    input.luid = luids[i];
                    input.vendor = match vendors[i] {
                        0 => NV,
                        1 => AMF,
                        2 => MFX,
                        _ => {
                            log::error!(
                                "Unexpected vendor value encountered: {}. Skipping.",
                                vendors[i]
                            );
                            continue;
                        },                    };
                    exclude_luid_formats.push((luids[i], input.data_format as i32));
                    outputs.push(input);
                }
            }
        } else {
            debug!(
                "vram decoder test failed: driver={:?}, error={}",
                input.driver, result
            );
        }
    }

    outputs
}
