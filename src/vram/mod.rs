mod amf_bridge;
mod mfx_bridge;
mod nv_bridge;

pub(crate) mod amf;
pub mod decode;
pub mod encode;
mod inner;
pub(crate) mod mfx;
pub(crate) mod nv;

// cxx 的 extern "Rust" 由各 *_bridge.rs 内同名函数实现，此处无需再包装

pub(crate) const MAX_ADATERS: usize = 16;

use crate::common::{DataFormat, Driver};
pub use serde;
pub use serde_derive;
use serde_derive::{Deserialize, Serialize};
use std::ffi::c_void;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FeatureContext {
    pub driver: Driver,
    pub vendor: Driver,
    pub luid: i64,
    pub data_format: DataFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct DynamicContext {
    #[serde(skip)]
    pub device: Option<*mut c_void>,
    pub width: i32,
    pub height: i32,
    pub kbitrate: i32,
    pub framerate: i32,
    pub gop: i32,
}

unsafe impl Send for DynamicContext {}
unsafe impl Sync for DynamicContext {}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct EncodeContext {
    pub f: FeatureContext,
    pub d: DynamicContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct DecodeContext {
    #[serde(skip)]
    pub device: Option<*mut c_void>,
    pub driver: Driver,
    pub vendor: Driver,
    pub luid: i64,
    pub data_format: DataFormat,
}

unsafe impl Send for DecodeContext {}
unsafe impl Sync for DecodeContext {}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Available {
    pub e: Vec<FeatureContext>,
    pub d: Vec<DecodeContext>,
}

impl Available {
    pub fn serialize(&self) -> Result<String, ()> {
        match serde_json::to_string_pretty(self) {
            Ok(s) => Ok(s),
            Err(_) => Err(()),
        }
    }

    pub fn deserialize(s: &str) -> Result<Self, ()> {
        match serde_json::from_str(s) {
            Ok(c) => Ok(c),
            Err(_) => Err(()),
        }
    }

    pub fn contains(&self, encode: bool, vendor: Driver, data_format: DataFormat) -> bool {
        if encode {
            self.e
                .iter()
                .any(|f| f.vendor == vendor && f.data_format == data_format)
        } else {
            self.d
                .iter()
                .any(|d| d.vendor == vendor && d.data_format == data_format)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::DataFormat;
    
    /// 测试 FeatureContext 结构体
    #[test]
    fn test_feature_context() {
        let context = FeatureContext {
            driver: Driver::NV,
            vendor: Driver::NV,
            luid: 12345,
            data_format: DataFormat::H264,
        };
        
        assert_eq!(context.driver, Driver::NV);
        assert_eq!(context.vendor, Driver::NV);
        assert_eq!(context.luid, 12345);
        assert_eq!(context.data_format, DataFormat::H264);
    }
    
    /// 测试 DynamicContext 结构体
    #[test]
    fn test_dynamic_context() {
        let context = DynamicContext {
            device: None,
            width: 1920,
            height: 1080,
            kbitrate: 5000,
            framerate: 30,
            gop: 60,
        };
        
        assert_eq!(context.width, 1920);
        assert_eq!(context.height, 1080);
        assert_eq!(context.kbitrate, 5000);
        assert_eq!(context.framerate, 30);
        assert_eq!(context.gop, 60);
    }
    
    /// 测试 EncodeContext 结构体
    #[test]
    fn test_encode_context() {
        let feature_context = FeatureContext {
            driver: Driver::NV,
            vendor: Driver::NV,
            luid: 12345,
            data_format: DataFormat::H264,
        };
        
        let dynamic_context = DynamicContext {
            device: None,
            width: 1920,
            height: 1080,
            kbitrate: 5000,
            framerate: 30,
            gop: 60,
        };
        
        let context = EncodeContext {
            f: feature_context,
            d: dynamic_context,
        };
        
        assert_eq!(context.f.driver, Driver::NV);
        assert_eq!(context.d.width, 1920);
    }
    
    /// 测试 DecodeContext 结构体
    #[test]
    fn test_decode_context() {
        let context = DecodeContext {
            device: None,
            driver: Driver::NV,
            vendor: Driver::NV,
            luid: 12345,
            data_format: DataFormat::H264,
        };
        
        assert_eq!(context.driver, Driver::NV);
        assert_eq!(context.vendor, Driver::NV);
        assert_eq!(context.luid, 12345);
        assert_eq!(context.data_format, DataFormat::H264);
    }
    
    /// 测试 Available 结构体
    #[test]
    fn test_available() {
        let feature_context = FeatureContext {
            driver: Driver::NV,
            vendor: Driver::NV,
            luid: 12345,
            data_format: DataFormat::H264,
        };
        
        let decode_context = DecodeContext {
            device: None,
            driver: Driver::NV,
            vendor: Driver::NV,
            luid: 12345,
            data_format: DataFormat::H264,
        };
        
        let available = Available {
            e: vec![feature_context],
            d: vec![decode_context],
        };
        
        assert_eq!(available.e.len(), 1);
        assert_eq!(available.d.len(), 1);
        
        // 测试 contains 方法
        assert!(available.contains(true, Driver::NV, DataFormat::H264));
        assert!(available.contains(false, Driver::NV, DataFormat::H264));
        assert!(!available.contains(true, Driver::AMF, DataFormat::H265));
    }
    
    /// 测试 Available 序列化和反序列化
    #[test]
    fn test_available_serialization() {
        let feature_context = FeatureContext {
            driver: Driver::NV,
            vendor: Driver::NV,
            luid: 12345,
            data_format: DataFormat::H264,
        };
        
        let decode_context = DecodeContext {
            device: None,
            driver: Driver::NV,
            vendor: Driver::NV,
            luid: 12345,
            data_format: DataFormat::H264,
        };
        
        let available = Available {
            e: vec![feature_context],
            d: vec![decode_context],
        };
        
        // 测试序列化
        let serialized = available.serialize();
        assert!(serialized.is_ok());
        
        // 测试反序列化
        if let Ok(serialized_str) = serialized {
            let deserialized = Available::deserialize(&serialized_str);
            assert!(deserialized.is_ok());
        }
    }
}
