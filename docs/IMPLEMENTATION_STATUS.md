# 实现状态（当前架构）

## 概述

本文档描述基于 **cxx bridge** 的当前实现状态：Rust 侧调用链与类型已就绪，C++ 侧位于 `cpp/` 目录，**驱动检测、test_encode/test_decode、NV/AMF/MFX 编解码已实现**（AMF 需 externals/AMF_v1.4.35；MFX 需系统有 mfx.dll 或 libmfxhw64.dll 及 Intel Media SDK 运行时）。

**最后更新**：2026-02

**当前状态**：
- ✅ 构建与 Rust/C++ 桥接：完成（build.rs + cxx，无 bindgen，无 feature 条件；MSVC 下为 AMF/MFX 自动添加 VC 与 Windows Kits include 路径）
- ✅ Rust 编码/解码调用链：完成（encode.rs/decode.rs → amf/nv/mfx.rs → *_bridge）
- ✅ 驱动/能力检测：C++ 通过 LoadLibrary 检测 amfrt64.dll / nvEncodeAPI64.dll&nvcuvid.dll / mfx.dll，Rust 通过 bridge 调用并实现 *\_driver_support
- ✅ test_encode / test_decode：按 (luid, format) 排除，同一适配器可同时上报 H264 与 H265；Rust 中按驱动可用性填写 desc_count、luids、vendors
- ✅ **NV C++ 编解码**：nv_bridge.cpp 仅用 nv-codec-headers 运行时 dynlink（无编译期 CUDA 依赖）。编码 H.264/H.265（NVENC）；解码 H.264/H.265（NVDEC，cuvid 解析+解码+D3D11 NV12 纹理输出）已实现。
- ✅ **AMF 编码/解码**：`cpp/amf_bridge.cpp` 在存在 **externals/AMF_v1.4.35** 时启用 H.264 + H.265 编码（VCE_AVC / AMFVideoEncoder_HEVC）与 H.264 + H.265 解码（UVD_H264_AVC / HW_H265_HEVC）；不存在时为占位。
- ✅ **MFX C++ 编解码**：`cpp/mfx_bridge.cpp` 动态加载 mfx.dll/libmfxhw64.dll，H.264/H.265 编码与解码完整实现（CodecId AVC/HEVC，D3D11 + NV12）。
- 📌 **示例**：`color_to_h264`、`color_to_h265`（NV/AMF/MFX 按可用性选用；示例内带调试日志与“纹理与编码器须使用同一 D3D11 设备”提示）。

---

## 当前架构

### 构建（build.rs）

- 使用 **cxx_build** 为 NV/AMF/MFX 各编译一个 bridge：
  - Rust 定义：`src/vram/nv_bridge.rs`、`amf_bridge.rs`、`mfx_bridge.rs`
  - C++ 实现：`cpp/nv_bridge.cpp`、`cpp/amf_bridge.cpp`、`cpp/mfx_bridge.cpp`
  - 头文件：`cpp/*_bridge.h`
- Include 路径：`cpp/` 与各 SDK 在 `externals/` 下的路径（Video_Codec_SDK、AMF_v1.4.35、MediaSDK_22.5.4）
- **无** feature 条件：三个 bridge 始终参与编译

### Rust 调用链

```
encode.rs / decode.rs
  → EncodeCalls / DecodeCalls（函数指针）
  → nv.rs / amf.rs / mfx.rs（提供 new/encode/decode/destroy/test/set_bitrate/set_framerate）
  → nv_bridge / amf_bridge / mfx_bridge（cxx 生成）
  → cpp/*_bridge.cpp（NV/AMF/MFX 均已接入对应 SDK）
```

### 文件结构

```
cpp/
├── amf_bridge.h
├── amf_bridge.cpp      # 存在 AMF_v1.4.35 时 H.264/H.265 编解码完整实现；否则占位
├── nv_bridge.h
├── nv_bridge.cpp       # NV dynlink：H.264/H.265 编码（NVENC）+ 解码（NVDEC→D3D11 NV12）
├── mfx_bridge.h
└── mfx_bridge.cpp      # 动态加载 mfx.dll/libmfxhw64.dll，Media SDK 编解码完整实现（D3D11 + NV12）

src/vram/
├── mod.rs
├── inner.rs            # EncodeCalls / DecodeCalls 类型定义
├── encode.rs           # Encoder，使用 EncodeCalls
├── decode.rs           # Decoder，使用 DecodeCalls
├── amf_bridge.rs       # cxx bridge 定义（AMF）
├── amf.rs              # AMF 的 new/encode/decode/destroy/test 等，调用 amf_bridge
├── nv_bridge.rs
├── nv.rs
├── mfx_bridge.rs
└── mfx.rs
```

---

## 各平台状态

### AMF (AMD)

| 项目 | 状态 | 说明 |
|------|------|------|
| Bridge 声明与类型 | ✅ | `amf_bridge.rs` + `cpp/amf_bridge.h` |
| Rust 侧封装 | ✅ | `amf.rs` 提供 encode_calls/decode_calls，调用 amf_CreateEncoder、amf_EncodeFrame 等 |
| C++ 实现 | ✅ 编码 + 解码 | H.264/H.265 编码（VCE_AVC + AMFVideoEncoder_HEVC）与 H.264/H.265 解码（UVD_AVC + HW_HEVC），需 externals/AMF_v1.4.35 |
| driver_support | ✅ | C++ `amf_IsDriverAvailable()` 检测 amfrt64.dll；Rust `amf_driver_support()` 调用 bridge |
| test_encode / test_decode | ✅ | Rust 中按驱动可用性填写 desc_count、luids、vendors（vendor=1） |

### NVIDIA

| 项目 | 状态 | 说明 |
|------|------|------|
| Bridge 声明与类型 | ✅ | `nv_bridge.rs` + `cpp/nv_bridge.h` |
| Rust 侧封装 | ✅ | `nv.rs` 调用 nv_CreateEncoder、nv_EncodeFrame 等 |
| C++ 实现 | ✅ | nv_bridge.cpp 仅 dynlink（无 CUDA 编译依赖）：H.264/H.265 编码（NVENC）+ H.264/H.265 解码（NVDEC→D3D11 NV12） |
| encode/decode driver_support | ✅ | C++ `nv_IsEncodeDriverAvailable`/`nv_IsDecodeDriverAvailable` 检测 nvEncodeAPI64.dll、nvcuvid.dll |
| test_encode / test_decode | ✅ | Rust 中按驱动可用性填写 desc_count、luids、vendors（vendor=0） |

### MFX (Intel)

| 项目 | 状态 | 说明 |
|------|------|------|
| Bridge 声明与类型 | ✅ | `mfx_bridge.rs` + `cpp/mfx_bridge.h` |
| Rust 侧封装 | ✅ | `mfx.rs` 调用 mfx_CreateEncoder、mfx_EncodeFrame 等 |
| C++ 实现 | ✅ | 动态加载 mfx.dll/libmfxhw64.dll，H.264/H.265 编解码完整实现（AVC/HEVC profile/level，D3D11 + NV12） |
| driver_support | ✅ | C++ `mfx_IsDriverAvailable()` 检测 mfx.dll；Rust 通过 bridge 的 encode/decode_driver_support |
| test_encode / test_decode | ✅ | Rust 中按驱动可用性填写 desc_count、luids、vendors（vendor=2） |

---

## Windows 平台基础设施

| 功能 | 状态 | 说明 |
|------|------|------|
| 纹理宽高 | ✅ | `src/platform/win/ffi.rs::hwcodec_get_d3d11_texture_width_height`（Rust 实现） |
| 适配器 / 设备 | ✅ | `src/platform/win/adapter.rs`、`device.rs` |
| 工具函数 | ✅ | `src/platform/win/utils.rs`（get_gpu_signature、add_process_to_new_job 等） |

---

## 待实现（可选/按需）

1. **高级行为**（按需）：分辨率变化处理、Drain、转换器与颜色空间（AMF）等，参考 `CPP_RUST_DIFFERENCES.md` 中的“待补齐方向”。

---

## 参考文档

- **CPP_VS_RUST_AND_CLEANUP.md**：当前构建与调用关系、冗余代码删除记录
- **CPP_RUST_DIFFERENCES.md**：当前架构说明及与“完整行为”的差异
- **REFACTORING_PLAN.md**：重构目标与已完成的构建/桥接变更
