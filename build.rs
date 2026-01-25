use cc::Build;
use std::{
    env,
    path::{Path, PathBuf},
};

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let externals_dir = manifest_dir.join("externals");
    let cpp_dir = manifest_dir.join("cpp");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=deps");
    println!("cargo:rerun-if-changed={}", externals_dir.display());
    println!("cargo:rerun-if-changed={}", cpp_dir.display());
    
    // 配置 cxx bridge（Windows 平台）
    // 注意：由于 AMF SDK 的复杂性（智能指针、COM-like 接口），
    // 我们暂时不使用 cxx bridge，而是通过动态加载 DLL 和函数指针的方式调用 AMF SDK
    // 这样可以最大程度减少 C++ 代码，所有业务逻辑在 Rust 中实现
    // 
    // 如果未来需要使用 cxx bridge，可以这样配置：
    // #[cfg(windows)]
    // {
    //     let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    //     let externals_dir = manifest_dir.join("externals");
    //     let amf_path = externals_dir.join("AMF_v1.4.35");
    //     cxx_build::bridge("src/vram/amf_rust.rs")
    //         .flag_if_supported("-std=c++17")
    //         .include(format!("{}/amf/public/common", amf_path.display()))
    //         .include(amf_path.join("amf"))
    //         .include(manifest_dir.join("cpp").join("amf"))
    //         .include(manifest_dir.join("cpp").join("common"))
    //         .compile("hwcodec-amf-cxx");
    // }
    
    let mut builder = Build::new();

    build_common(&mut builder);
    #[cfg(windows)]
    sdk::build_sdk(&mut builder);
    builder
        .static_crt(true)
        .flag("/EHsc")
        .flag("/utf-8")
        .define("_CRT_SECURE_NO_WARNINGS", None)
        .compile("hwcodec");
}

fn build_common(builder: &mut Build) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let common_dir = manifest_dir.join("cpp").join("common");
    bindgen::builder()
        .header(common_dir.join("common.h").to_string_lossy().to_string())
        .header(common_dir.join("callback.h").to_string_lossy().to_string())
        .rustified_enum("*")
        .parse_callbacks(Box::new(CommonCallbacks))
        .generate()
        .unwrap()
        .write_to_file(Path::new(&env::var_os("OUT_DIR").unwrap()).join("common_ffi.rs"))
        .unwrap();

    // system
    #[cfg(windows)]
    {
        for lib in ["d3d11", "dxgi"] {
            println!("cargo:rustc-link-lib={}", lib);
        }
    }

    builder.include(&common_dir);

    // platform
    let _platform_path = common_dir.join("platform");
    #[cfg(windows)]
    {
        let win_path = _platform_path.join("win");
        builder.include(&win_path);
        // NOTE: win.cpp, bmp.cpp, dump.cpp 的所有功能已完全替换为 Rust 实现
        // C++ 代码现在直接使用 Rust FFI 接口（win_rust_ffi.h）
        // 不再需要编译这些 C++ 文件
        // builder.file(win_path.join("win.cpp"));
        // builder.file(win_path.join("bmp.cpp"));
        // builder.file(win_path.join("dump.cpp"));
    }
    #[cfg(target_os = "linux")]
    {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let externals_dir = manifest_dir.join("externals");
        // ffnvcodec
        let ffnvcodec_path = externals_dir
            .join("nv-codec-headers_n12.1.14.0")
            .join("include")
            .join("ffnvcodec");
        builder.include(ffnvcodec_path);

        let linux_path = _platform_path.join("linux");
        builder.include(&linux_path);
        builder.file(linux_path.join("linux.cpp"));
    }
    if target_os == "macos" {
        let macos_path = _platform_path.join("mac");
        builder.include(&macos_path);
        builder.file(macos_path.join("mac.mm"));
    }

    // tool
    builder.files(["log.cpp", "util.cpp"].map(|f| common_dir.join(f)));
}

#[derive(Debug)]
struct CommonCallbacks;
impl bindgen::callbacks::ParseCallbacks for CommonCallbacks {
    fn add_derives(&self, name: &str) -> Vec<String> {
        let names = vec!["DataFormat", "SurfaceFormat", "API"];
        if names.contains(&name) {
            vec!["Serialize", "Deserialize"]
                .drain(..)
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![]
        }
    }
}

#[cfg(windows)]
mod sdk {
    use super::*;

    pub(crate) fn build_sdk(builder: &mut Build) {
        build_amf(builder);
        build_nv(builder);
        build_mfx(builder);
    }


    fn build_nv(builder: &mut Build) {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let externals_dir = manifest_dir.join("externals");
        let common_dir = manifest_dir.join("common");
        let nv_dir = manifest_dir.join("cpp").join("nv");
        println!("cargo:rerun-if-changed=src");
        println!("cargo:rerun-if-changed={}", common_dir.display());
        println!("cargo:rerun-if-changed={}", externals_dir.display());
        bindgen::builder()
            .header(&nv_dir.join("nv_ffi.h").to_string_lossy().to_string())
            .rustified_enum("*")
            .generate()
            .unwrap()
            .write_to_file(Path::new(&env::var_os("OUT_DIR").unwrap()).join("nv_ffi.rs"))
            .unwrap();

        // system
        #[cfg(target_os = "windows")]
        {
            for lib in [
                "kernel32", "user32", "gdi32", "winspool", "shell32", "ole32", "oleaut32", "uuid",
                "comdlg32", "advapi32", "d3d11", "dxgi",
            ] {
                println!("cargo:rustc-link-lib={}", lib);
            }
        }
        #[cfg(target_os = "linux")]
        println!("cargo:rustc-link-lib=stdc++");

        // ffnvcodec
        let ffnvcodec_path = externals_dir
            .join("nv-codec-headers_n12.1.14.0")
            .join("include")
            .join("ffnvcodec");
        builder.include(ffnvcodec_path);

        // video codc sdk
        let sdk_path = externals_dir.join("Video_Codec_SDK_12.1.14");
        builder.includes([
            sdk_path.clone(),
            sdk_path.join("Interface"),
            sdk_path.join("Samples").join("Utils"),
            sdk_path.join("Samples").join("NvCodec"),
            sdk_path.join("Samples").join("NvCodec").join("NVEncoder"),
            sdk_path.join("Samples").join("NvCodec").join("NVDecoder"),
        ]);

        for file in vec!["NvEncoder.cpp", "NvEncoderD3D11.cpp"] {
            builder.file(
                sdk_path
                    .join("Samples")
                    .join("NvCodec")
                    .join("NvEncoder")
                    .join(file),
            );
        }
        for file in vec!["NvDecoder.cpp"] {
            builder.file(
                sdk_path
                    .join("Samples")
                    .join("NvCodec")
                    .join("NvDecoder")
                    .join(file),
            );
        }

        // NOTE: Windows平台使用 nv_wrapper.cpp 作为 C 包装层
        // nv_encode.cpp 和 nv_decode.cpp 已完全替换为 Rust 实现
        #[cfg(windows)]
        {
            // Windows 平台：编译 nv_wrapper.cpp 作为 C 包装层
            builder.file(nv_dir.join("nv_wrapper.cpp"));
            
            // 生成 nv_wrapper FFI 绑定
            bindgen::builder()
                .header(nv_dir.join("nv_wrapper.h").to_string_lossy().to_string())
                .allowlist_function("nv_wrapper_.*")
                .blocklist_type(".*")
                .blocklist_item(".*")
                .parse_callbacks(Box::new(bindgen::CargoCallbacks))
                .generate()
                .unwrap()
                .write_to_file(Path::new(&env::var_os("OUT_DIR").unwrap()).join("nv_wrapper_ffi.rs"))
                .unwrap();
        }
        #[cfg(not(windows))]
        {
            // 非 Windows 平台：继续使用 C++ 实现
            builder.files(["nv_encode.cpp", "nv_decode.cpp"].map(|f| nv_dir.join(f)));
        }
    }

    fn build_amf(builder: &mut Build) {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let externals_dir = manifest_dir.join("externals");
        let amf_dir = manifest_dir.join("cpp").join("amf");
        println!("cargo:rerun-if-changed=src");
        println!("cargo:rerun-if-changed={}", externals_dir.display());
        bindgen::builder()
            .header(amf_dir.join("amf_ffi.h").to_string_lossy().to_string())
            .rustified_enum("*")
            .generate()
            .unwrap()
            .write_to_file(Path::new(&env::var_os("OUT_DIR").unwrap()).join("amf_ffi.rs"))
            .unwrap();

        // system
        #[cfg(windows)]
        println!("cargo:rustc-link-lib=ole32");
        #[cfg(target_os = "linux")]
        println!("cargo:rustc-link-lib=stdc++");

        // amf
        let amf_path = externals_dir.join("AMF_v1.4.35");
        builder.include(format!("{}/amf/public/common", amf_path.display()));
        builder.include(amf_path.join("amf"));

        for f in vec![
            "AMFFactory.cpp",
            "AMFSTL.cpp",
            "Thread.cpp",
            #[cfg(windows)]
            "Windows/ThreadWindows.cpp",
            #[cfg(target_os = "linux")]
            "Linux/ThreadLinux.cpp",
            "TraceAdapter.cpp",
        ] {
            builder.file(format!("{}/amf/public/common/{}", amf_path.display(), f));
        }

        // crate
        // NOTE: Windows平台使用 C 包装层（amf_wrapper.cpp）包装 SDK 调用
        // 所有业务逻辑在 Rust 中实现
        #[cfg(windows)]
        {
            // 编译 C 包装层
            builder.file(amf_dir.join("amf_wrapper.cpp"));
            
            // 生成 Rust 绑定
            // 只包含 amf_wrapper 函数，排除所有系统类型和常量
            bindgen::builder()
                .header(amf_dir.join("amf_wrapper.h").to_string_lossy().to_string())
                // 只允许 amf_wrapper_ 开头的函数
                .allowlist_function("amf_wrapper_.*")
                // 排除所有系统头文件中的定义
                .blocklist_file(".*stdint\\.h.*")
                .blocklist_file(".*stddef\\.h.*")
                .blocklist_file(".*sal\\.h.*")
                .blocklist_file(".*crtdefs\\.h.*")
                .blocklist_file(".*vcruntime\\.h.*")
                .blocklist_file(".*corecrt\\.h.*")
                .blocklist_file(".*stdarg\\.h.*")
                // 排除系统常量和类型，但允许基本整数类型
                .blocklist_type(".*")
                .allowlist_type("int.*")
                .allowlist_type("uint.*")
                .allowlist_type("size_t")
                .allowlist_type("wchar_t")
                // 先 blocklist 所有函数，再 allowlist amf_wrapper_ 函数
                .blocklist_function(".*")
                .allowlist_function("amf_wrapper_.*")
                // 使用 C 调用约定
                .use_core()
                .ctypes_prefix("::std::os::raw")
                .generate()
                .unwrap()
                .write_to_file(Path::new(&env::var_os("OUT_DIR").unwrap()).join("amf_wrapper_ffi.rs"))
                .unwrap();
        }
        #[cfg(not(windows))]
        {
            builder.files(["amf_encode.cpp", "amf_decode.cpp"].map(|f| amf_dir.join(f)));
        }
    }

    fn build_mfx(builder: &mut Build) {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let externals_dir = manifest_dir.join("externals");
        let mfx_dir = manifest_dir.join("cpp").join("mfx");
        println!("cargo:rerun-if-changed=src");
        println!("cargo:rerun-if-changed={}", externals_dir.display());
        bindgen::builder()
            .header(&mfx_dir.join("mfx_ffi.h").to_string_lossy().to_string())
            .rustified_enum("*")
            .generate()
            .unwrap()
            .write_to_file(Path::new(&env::var_os("OUT_DIR").unwrap()).join("mfx_ffi.rs"))
            .unwrap();

        // MediaSDK
        let sdk_path = externals_dir.join("MediaSDK_22.5.4");

        // mfx_dispatch
        let mfx_path = sdk_path.join("api").join("mfx_dispatch");
        let mfx_dispatch_src = mfx_path.join("windows").join("src");
        // include headers and compile source files
        builder.include(mfx_path.join("windows").join("include"));
        
        // 为所有 MFX 相关文件添加必要的定义
        builder
            .define("NOMINMAX", None)
            .define("MFX_DEPRECATED_OFF", None)
            .define("MFX_D3D11_SUPPORT", None);
        
        // compile mfx_dispatch source files
        builder.files([
            "mfx_dispatcher.cpp",
            "mfx_dispatcher_main.cpp",
            "mfx_function_table.cpp",
            "mfx_library_iterator.cpp",
            "mfx_load_dll.cpp",
            "mfx_load_plugin.cpp",
            "mfx_plugin_hive.cpp",
            "mfx_win_reg_key.cpp",
            "mfx_driver_store_loader.cpp",
            "mfx_dxva2_device.cpp",
            "mfx_dispatcher_log.cpp",
            "mfx_critical_section.cpp",
        ].map(|f| mfx_dispatch_src.join(f)));

        let sample_path = sdk_path.join("samples").join("sample_common");
        builder
            .includes([
                sdk_path.join("api").join("include"),
                sample_path.join("include"),
            ])
            .files(
                [
                    "sample_utils.cpp",
                    "base_allocator.cpp",
                    "d3d11_allocator.cpp",
                    "avc_bitstream.cpp",
                    "avc_spl.cpp",
                    "avc_nal_spl.cpp",
                ]
                .map(|f| sample_path.join("src").join(f)),
            )
            .files(
                [
                    "time.cpp",
                    "atomic.cpp",
                    "shared_object.cpp",
                    "thread_windows.cpp",
                ]
                .map(|f| sample_path.join("src").join("vm").join(f)),
            );

        // link
        {
            for lib in [
                "kernel32", "user32", "gdi32", "winspool", "shell32", "ole32", "oleaut32", "uuid",
                "comdlg32", "advapi32", "d3d11", "dxgi",
            ] {
                println!("cargo:rustc-link-lib={}", lib);
            }
        }

        // NOTE: Windows平台使用 mfx_wrapper.cpp 作为 C 包装层
        // mfx_encode.cpp 和 mfx_decode.cpp 已完全替换为 Rust 实现
        #[cfg(windows)]
        {
            // Windows 平台：编译 mfx_wrapper.cpp 作为 C 包装层
            builder.file(mfx_dir.join("mfx_wrapper.cpp"));
            
            // 生成 mfx_wrapper FFI 绑定
            bindgen::builder()
                .header(mfx_dir.join("mfx_wrapper.h").to_string_lossy().to_string())
                .allowlist_function("mfx_wrapper_.*")
                .blocklist_type(".*")
                .blocklist_item(".*")
                .allowlist_type("mfx.*")
                .parse_callbacks(Box::new(bindgen::CargoCallbacks))
                .generate()
                .unwrap()
                .write_to_file(Path::new(&env::var_os("OUT_DIR").unwrap()).join("mfx_wrapper_ffi.rs"))
                .unwrap();
        }
        #[cfg(not(windows))]
        {
            // 非 Windows 平台：继续使用 C++ 实现
            builder
                .files(["mfx_encode.cpp", "mfx_decode.cpp"].map(|f| mfx_dir.join(f)))
                .define("NOMINMAX", None)
                .define("MFX_DEPRECATED_OFF", None)
                .define("MFX_D3D11_SUPPORT", None);
        }
    }
}
