//! 按 REFACTORING_PLAN：统一 cxx、直接连接 externals SDK、简化构建、减少对 cpp/ 的依赖
//!
//! **约定：禁止修改 externals/ 下任何 SDK 目录**（只读 include，不复制、不写入）。
//! SDK 可能随时更新，对 externals 的修改会在更新后丢失。

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let externals_dir = manifest_dir.join("externals");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cpp_dir = manifest_dir.join("cpp");

    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cpp");
    println!("cargo:rerun-if-changed={}", externals_dir.display());

    // 生成 common_ffi.rs 占位（不再使用 bindgen，保留以通过 include!）
    let common_ffi_path = out_dir.join("common_ffi.rs");
    std::fs::write(
        &common_ffi_path,
        "// Placeholder: common FFI (was bindgen-generated)\npub const DECODE_TIMEOUT_MS: u32 = 1000;\n",
    )
    .expect("write common_ffi.rs");

    let is_msvc = cfg!(target_env = "msvc");

    // --- NV: 仅用 nv-codec-headers，编码/解码均在运行时通过 LoadLibrary 动态检测，不依赖 CUDA/Video_Codec_SDK 编译 ---
    let nv_bridge_h = cpp_dir.join("nv_bridge.h");
    let mut nv_bridge = cxx_build::bridge("src/vram/nv_bridge.rs");
    let mut nv = nv_bridge.file("cpp/nv_bridge.cpp").include(&cpp_dir);
    if let Some(inc) = externals_dir.join("nv-codec-headers_n12.1.14.0/include").to_str() {
        nv = nv.include(inc);
    }
    if !is_msvc {
        nv = nv.flag_if_supported("-std=c++17");
    }
    if nv_bridge_h.exists() {
        nv = if is_msvc { nv.flag("/FInv_bridge.h") } else { nv.flag("-include").flag("nv_bridge.h") };
    }
    nv.compile("hwcodec-nv");

    // --- AMF: 存在 externals/AMF_v1.4.35 时连接 AMF SDK 完整实现，否则为占位 ---
    let amf_bridge_h = cpp_dir.join("amf_bridge.h");
    let amf_root = externals_dir.join("AMF_v1.4.35");
    let mut amf_bridge = cxx_build::bridge("src/vram/amf_bridge.rs");
    let mut amf = amf_bridge.file("cpp/amf_bridge.cpp").include(&cpp_dir);
    if amf_root.exists() {
        amf = amf.define("HWCODEC_AMF_FULL", None);
        // 确保 MSVC 标准库头路径在 -I 前，避免 /FI 下找不到 cstdint/cstdlib 等
        if is_msvc {
            let mut vc_include: Option<PathBuf> = None;
            if let Ok(vc) = env::var("VCToolsInstallDir") {
                let p = PathBuf::from(vc.trim_end_matches(|c| c == '/' || c == '\\')).join("include");
                if p.exists() {
                    vc_include = Some(p);
                }
            }
            if vc_include.is_none() {
                let pf = env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());
                let msvc_base = PathBuf::from(&pf).join("Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC");
                if msvc_base.exists() {
                    if let Ok(entries) = std::fs::read_dir(&msvc_base) {
                        let mut versions: Vec<PathBuf> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| e.path().join("include"))
                            .filter(|p| p.exists())
                            .collect();
                        versions.sort_by(|a, b| b.cmp(a));
                        if let Some(p) = versions.into_iter().next() {
                            vc_include = Some(p);
                        }
                    }
                }
            }
            if let Some(p) = vc_include.as_ref().and_then(|p| p.to_str()) {
                amf = amf.include(p);
            }
            // UCRT (math.h 等) 在 Windows Kits 下
            let kits = env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());
            let kits_include = PathBuf::from(&kits).join("Windows Kits/10/Include");
            if kits_include.exists() {
                if let Ok(entries) = std::fs::read_dir(&kits_include) {
                    let mut vers: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
                    vers.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
                    for ver in vers {
                        for sub in &["ucrt", "shared", "um"] {
                            let p = ver.join(sub);
                            if p.exists() {
                                if let Some(s) = p.to_str() {
                                    amf = amf.include(s);
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
        if let Some(p) = amf_root.join("amf/public/include").to_str() {
            amf = amf.include(p);
        }
        if let Some(p) = amf_root.join("amf/public/common").to_str() {
            amf = amf.include(p);
        }
    }
    if !is_msvc {
        amf = amf.flag_if_supported("-std=c++17");
    }
    if amf_bridge_h.exists() {
        amf = if is_msvc { amf.flag("/FIamf_bridge.h") } else { amf.flag("-include").flag("amf_bridge.h") };
    }
    if is_msvc {
        amf = amf.flag("/utf-8"); // 源文件含中文，避免 C4819/C2001
    }
    amf.compile("hwcodec-amf");

    // --- MFX: 直接连接 externals MediaSDK_22.5.4 ---
    let mfx_bridge_h = cpp_dir.join("mfx_bridge.h");
    let mfx_root = externals_dir.join("MediaSDK_22.5.4");
    let mut mfx_bridge = cxx_build::bridge("src/vram/mfx_bridge.rs");
    let mut mfx = mfx_bridge.file("cpp/mfx_bridge.cpp").include(&cpp_dir);
    if mfx_root.exists() {
        if let Some(p) = mfx_root.join("api/include").to_str() {
            mfx = mfx.include(p);
        }
        // MSVC: 确保标准库和 Windows SDK 头路径可用（mfx 会 include d3d11.h / cstdlib 等）
        if is_msvc {
            let mut vc_include: Option<PathBuf> = None;
            if let Ok(vc) = env::var("VCToolsInstallDir") {
                let p = PathBuf::from(vc.trim_end_matches(|c| c == '/' || c == '\\')).join("include");
                if p.exists() {
                    vc_include = Some(p);
                }
            }
            if vc_include.is_none() {
                let pf = env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());
                let msvc_base = PathBuf::from(&pf).join("Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC");
                if msvc_base.exists() {
                    if let Ok(entries) = std::fs::read_dir(&msvc_base) {
                        let mut versions: Vec<PathBuf> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| e.path().join("include"))
                            .filter(|p| p.exists())
                            .collect();
                        versions.sort_by(|a, b| b.cmp(a));
                        if let Some(p) = versions.into_iter().next() {
                            vc_include = Some(p);
                        }
                    }
                }
            }
            if let Some(p) = vc_include.as_ref().and_then(|p| p.to_str()) {
                mfx = mfx.include(p);
            }
            let kits = env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());
            let kits_include = PathBuf::from(&kits).join("Windows Kits/10/Include");
            if kits_include.exists() {
                if let Ok(entries) = std::fs::read_dir(&kits_include) {
                    let mut vers: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
                    vers.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
                    for ver in vers {
                        for sub in &["ucrt", "shared", "um"] {
                            let p = ver.join(sub);
                            if p.exists() {
                                if let Some(s) = p.to_str() {
                                    mfx = mfx.include(s);
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    if !is_msvc {
        mfx = mfx.flag_if_supported("-std=c++17");
    }
    if mfx_bridge_h.exists() {
        mfx = if is_msvc { mfx.flag("/FImfx_bridge.h") } else { mfx.flag("-include").flag("mfx_bridge.h") };
    }
    mfx.compile("hwcodec-mfx");

    #[cfg(windows)]
    {
        println!("cargo:rustc-link-lib=d3d11");
        println!("cargo:rustc-link-lib=dxgi");
        println!("cargo:rustc-link-lib=ole32");
    }
}
