# C++ 到 Rust 迁移实现状态

## 概述

本文档记录从 C++ 实现迁移到 Rust 实现的完整状态，包括实现方案、各 SDK 的详细状态、待实现功能清单和实现进度。

**实现策略**：
- Windows 平台：使用 C 包装层（`*_wrapper.cpp`）+ Rust 业务逻辑
- 非 Windows 平台：继续使用 C++ 实现（通过 FFI）

**最后更新**：2025-01-25

**状态**：✅ **所有核心功能已完成** - AMF、MFX、NVIDIA 三个 SDK 均已达到 100% 完成度

**最新更新**：已修复所有 C++ 与 Rust 实现差异，包括：
- ✅ AMF 解码器分辨率变化处理（`amf_decode()` 现在正确处理 `AMF_RESOLUTION_CHANGED`）
- ✅ AMF Drain 操作（在销毁前调用 `Drain()` 排空待处理帧）
- ✅ AMF 转换器颜色空间属性（添加了完整的颜色空间属性设置）
- ✅ NVIDIA 解码器重新创建逻辑（当检测到 `-2` 时自动重新创建解码器并重新解码）

---

## 实现方案

### 最终选择：C 包装层 + Rust 业务逻辑

- **C 包装层**：只包装 externals SDK 的调用，提供简单 C 接口
- **业务逻辑**：全部在 Rust 中实现

### 方案对比

| 对比项 | 原方案（amf_encode.cpp） | 新方案（C包装层+Rust） |
|--------|------------------------|---------------------|
| C++代码量 | ~735行（包含业务逻辑） | ~520行（只包装SDK调用） |
| 业务逻辑位置 | C++中 | Rust中 ✅ |
| Rust掌控度 | ⭐⭐ | ⭐⭐⭐⭐⭐ ✅ |
| 适合Rust开发者 | ❌ | ✅ |

### C 包装层职责

- **只包装 SDK 调用** - 提供简单的 C 接口
- **不包含业务逻辑** - 所有业务逻辑在 Rust 中实现
- **错误处理** - 将 C++ 异常转换为返回码（0=成功, -1=失败, 1=需要更多输入）

### Rust 业务逻辑职责

- **编码器/解码器创建** - 调用 C 包装层，实现完整流程
- **参数设置** - 所有编码参数设置逻辑
- **编码/解码流程** - 完整的编码/解码流程控制
- **资源管理** - 生命周期管理

---

## AMF SDK (AMD)

### 文件结构

**C++ 文件**：
- `cpp/amf/amf_encode.cpp` - 编码器实现（~735行，Windows不编译）
- `cpp/amf/amf_decode.cpp` - 解码器实现（~501行，Windows不编译）
- `cpp/amf/amf_wrapper.cpp` - C 包装层（~520行）✅ **已完成**
- `cpp/amf/amf_wrapper.h` - C 接口头文件（~130行）✅ **已完成**

**Rust 文件**：
- `src/vram/amf_rust.rs` - Windows 平台 Rust 实现

### 实现状态

#### C 包装层状态 ✅ 100%

| C++ 功能 | C 包装层函数 | 状态 |
|---------|------------|------|
| Factory 初始化 | `amf_wrapper_factory_init()` | ✅ 已实现 |
| Context 创建和 DX11 初始化 | `amf_wrapper_create_context()`, `amf_wrapper_context_init_dx11()` | ✅ 已实现 |
| 编码器组件创建 | `amf_wrapper_create_encoder_component()` | ✅ 已实现 |
| 属性设置（码率、帧率、GOP等） | `amf_wrapper_component_set_property_*()` | ✅ 已实现 |
| 编码器初始化 | `amf_wrapper_component_init()` | ✅ 已实现 |
| Surface 创建（从 D3D11 纹理） | `amf_wrapper_create_surface_from_dx11()` | ✅ 已实现 |
| 提交输入 | `amf_wrapper_encoder_submit_input()` | ✅ 已实现 |
| 查询输出 | `amf_wrapper_encoder_query_output()` | ✅ 已实现 |
| 格式转换（BGRA ↔ NV12） | `amf_wrapper_create_converter_component()` | ✅ 已实现 |
| 解码器组件创建 | `amf_wrapper_create_decoder_component()` | ✅ 已实现 |
| 输入缓冲区创建 | `amf_wrapper_decoder_submit_input()` | ✅ 已实现（支持分辨率变化错误码） |
| 查询输出 Surface | `amf_wrapper_decoder_query_output()` | ✅ 已实现 |
| 组件 Drain 操作 | `amf_wrapper_component_drain()` | ✅ 已实现 |
| 从主机内存创建 Buffer | `amf_wrapper_create_buffer_from_host()` | ✅ 已实现 |

#### Rust 业务逻辑状态 ✅ 100%

| C++ 函数 | Rust 函数 | 状态 | 说明 |
|---------|----------|------|------|
| `amf_driver_support()` | `amf_driver_support()` | ✅ **已实现** | 动态加载 DLL，调用 AMFInit |
| `amf_new_encoder()` | `amf_new_encoder()` | ✅ **已实现** | 完整实现 |
| `amf_encode()` | `amf_encode()` | ✅ **已实现** | 完整实现（包括关键帧检测） |
| `amf_decode()` | `amf_decode()` | ✅ **已实现** | 完整实现（包括分辨率变化处理和颜色空间属性） |
| `amf_destroy_encoder()` | `amf_destroy_encoder()` | ✅ **已实现** | 完整实现 |
| `amf_new_decoder()` | `amf_new_decoder()` | ✅ **已实现** | 完整实现 |
| `amf_decode()` | `amf_decode()` | ✅ **已实现** | 完整实现 |
| `amf_destroy_decoder()` | `amf_destroy_decoder()` | ✅ **已实现** | 完整实现 |
| `amf_set_bitrate()` | `amf_set_bitrate()` | ✅ **已实现** | 完整实现 |
| `amf_set_framerate()` | `amf_set_framerate()` | ✅ **已实现** | 完整实现 |
| `amf_test_encode()` | `amf_test_encode()` | ✅ **已实现** | 完整实现 |
| `amf_test_decode()` | `amf_test_decode()` | ✅ **已实现** | 完整实现 |

### ✅ 已完成功能

所有核心功能已实现，AMF SDK 已达到 100% 完成度。

**最新修复**:
- ✅ 解码器分辨率变化处理（`amf_decode()` 现在正确处理 `AMF_RESOLUTION_CHANGED`）
- ✅ Drain 操作（在销毁前调用 `Drain()` 排空待处理帧）
- ✅ 转换器颜色空间属性（添加了完整的颜色空间属性设置）

---

---

## NVIDIA SDK

### 文件结构

**C++ 文件**：
- `cpp/nv/nv_encode.cpp` - 编码器实现（~490行，Windows不编译）
- `cpp/nv/nv_decode.cpp` - 解码器实现（~700行，Windows不编译）
- `cpp/nv/nv_wrapper.cpp` - ✅ **已完成**（~600行）
- `cpp/nv/nv_wrapper.h` - ✅ **已完成**（~200行）

**Rust 文件**：
- `src/vram/nv_rust.rs` - Windows 平台 Rust 实现

### 实现状态

#### C 包装层状态 ✅ 100%

| C++ 功能 | C 包装层函数 | 状态 |
|---------|------------|------|
| CUDA/NVENC 库加载 | `nv_wrapper_load_encoder_driver()`, `nv_wrapper_free_encoder_driver()` | ✅ 已实现 |
| CUDA/NVDEC 库加载 | `nv_wrapper_load_decoder_driver()`, `nv_wrapper_free_decoder_driver()` | ✅ 已实现 |
| CUDA 初始化和设备获取 | `nv_wrapper_cuda_init()`, `nv_wrapper_cuda_get_device_from_d3d11()` | ✅ 已实现 |
| CUDA 上下文管理 | `nv_wrapper_cuda_create_context()`, `nv_wrapper_cuda_destroy_context()` | ✅ 已实现 |
| 编码器创建和配置 | `nv_wrapper_create_encoder()`, `nv_wrapper_destroy_encoder()` | ✅ 已实现 |
| 编码操作 | `nv_wrapper_encoder_encode()`, `nv_wrapper_encoder_get_frame()` | ✅ 已实现 |
| 解码器创建和配置 | `nv_wrapper_create_decoder()`, `nv_wrapper_destroy_decoder()` | ✅ 已实现 |
| 解码操作 | `nv_wrapper_decoder_decode()`, `nv_wrapper_decoder_get_frame()` | ✅ 已实现 |
| 编码器参数设置 | `nv_wrapper_encoder_set_bitrate()`, `nv_wrapper_encoder_set_framerate()` | ✅ 已实现 |
| CUDA 纹理注册 | `nv_wrapper_cuda_register_texture()`, `nv_wrapper_cuda_unregister_texture()` | ✅ 已实现 |

#### Rust 业务逻辑状态 ✅ 100%

| C++ 函数 | Rust 函数 | 状态 | 说明 |
|---------|----------|------|------|
| `nv_encode_driver_support()` | `nv_encode_driver_support()` | ✅ **已实现** | 动态加载 NVENC 库并检测支持 |
| `nv_decode_driver_support()` | `nv_decode_driver_support()` | ✅ **已实现** | 动态加载 NVDEC 库并检测支持 |
| `nv_new_encoder()` | `nv_new_encoder()` | ✅ **已实现** | 完整实现（驱动加载、CUDA 初始化、编码器创建） |
| `nv_encode()` | `nv_encode()` | ✅ **已实现** | 完整实现（编码流程、纹理处理） |
| `nv_destroy_encoder()` | `nv_destroy_encoder()` | ✅ **已实现** | 完整实现（资源清理） |
| `nv_new_decoder()` | `nv_new_decoder()` | ✅ **已实现** | 完整实现（驱动加载、CUDA 初始化、解码器创建） |
| `nv_decode()` | `nv_decode()` | ✅ **已实现** | 完整实现（包括纹理复制、着色器渲染和重新创建逻辑） |
| `nv_destroy_decoder()` | `nv_destroy_decoder()` | ✅ **已实现** | 完整实现（资源清理） |
| `nv_test_encode()` | `nv_test_encode()` | ✅ **已实现** | 完整实现 |
| `nv_test_decode()` | `nv_test_decode()` | ✅ **已实现** | 完整实现 |
| `nv_set_bitrate()` | `nv_set_bitrate()` | ✅ **已实现** | 完整实现 |
| `nv_set_framerate()` | `nv_set_framerate()` | ✅ **已实现** | 完整实现 |

### ✅ 已完成功能

所有核心功能已实现，NVIDIA SDK 已达到 100% 完成度。

**实现亮点**:
- ✅ 完整的 CUDA 纹理复制流程（R8 和 R8G8 纹理）
- ✅ 完整的 D3D11 着色器渲染管线（SRV、RTV、视口、采样器、着色器、顶点缓冲区）
- ✅ 解码器重新创建逻辑（当分辨率变化时自动重新创建并重新解码）
- ✅ 所有测试函数已实现

**参考**: 
- `cpp/nv/nv_decode.cpp:308-333` (copy_cuda_frame)
- `cpp/nv/nv_decode.cpp:570-589` (register_texture)
- `cpp/nv/nv_decode.cpp:335-341` (draw)
- `cpp/nv/nv_decode.cpp:399-451` (set_srv)
- `cpp/nv/nv_decode.cpp:453-469` (set_rtv)
- `cpp/nv/nv_decode.cpp:347-397` (decode_and_recreate)

---

## MFX SDK (Intel)

### 文件结构

**C++ 文件**：
- `cpp/mfx/mfx_encode.cpp` - 编码器实现（~719行，Windows不编译）
- `cpp/mfx/mfx_decode.cpp` - 解码器实现（~460行，Windows不编译）
- `cpp/mfx/mfx_wrapper.cpp` - ✅ **已完成**（~700行）
- `cpp/mfx/mfx_wrapper.h` - ✅ **已完成**（~255行）

**Rust 文件**：
- `src/vram/mfx_rust.rs` - Windows 平台 Rust 实现

### 实现状态

#### C 包装层状态 ✅ 100%

| C++ 功能 | C 包装层函数 | 状态 |
|---------|------------|------|
| Session 初始化 | `mfx_wrapper_session_init()` | ✅ 已实现 |
| D3D11 设备句柄设置 | `mfx_wrapper_session_set_handle_d3d11()` | ✅ 已实现 |
| 帧分配器设置 | `mfx_wrapper_session_set_frame_allocator()` | ✅ 已实现 |
| 编码器创建 | `mfx_wrapper_create_encoder()` | ✅ 已实现 |
| 编码器参数设置 | `mfx_wrapper_create_encoder_params()` | ✅ 已实现 |
| 编码器查询和初始化 | `mfx_wrapper_encoder_query_and_init()` | ✅ 已实现 |
| 编码操作 | `mfx_wrapper_encoder_encode_frame_async()` | ✅ 已实现 |
| 解码器创建 | `mfx_wrapper_create_decoder()` | ✅ 已实现 |
| 解码器初始化和 Surface 分配 | `mfx_wrapper_decoder_initialize_from_bitstream()` | ✅ 已实现 |
| 解码操作 | `mfx_wrapper_decoder_decode_frame_async()` | ✅ 已实现 |
| D3D11 帧分配器 | `mfx_wrapper_create_d3d11_frame_allocator()` | ✅ 已实现 |
| Surface 操作 | `mfx_wrapper_surface_*()` | ✅ 已实现 |
| Bitstream 操作 | `mfx_wrapper_bitstream_*()` | ✅ 已实现 |

#### Rust 业务逻辑状态 ✅ 100%

| C++ 函数 | Rust 函数 | 状态 | 说明 |
|---------|----------|------|------|
| `mfx_driver_support()` | `mfx_driver_support()` | ✅ **已实现** | 测试 Session 初始化 |
| `mfx_new_encoder()` | `mfx_new_encoder()` | ✅ **已实现** | 完整实现 |
| `mfx_encode()` | `mfx_encode()` | ✅ **已实现** | 完整实现 |
| `mfx_destroy_encoder()` | `mfx_destroy_encoder()` | ✅ **已实现** | 完整实现 |
| `mfx_new_decoder()` | `mfx_new_decoder()` | ✅ **已实现** | 完整实现 |
| `mfx_decode()` | `mfx_decode()` | ✅ **已实现** | 完整实现（已从 Surface 获取尺寸） |
| `mfx_destroy_decoder()` | `mfx_destroy_decoder()` | ✅ **已实现** | 完整实现 |
| `mfx_test_encode()` | `mfx_test_encode()` | ✅ **已实现** | 完整实现 |
| `mfx_test_decode()` | `mfx_test_decode()` | ✅ **已实现** | 完整实现 |
| `mfx_set_bitrate()` | `mfx_set_bitrate()` | ✅ **已实现** | 完整实现 |
| `mfx_set_framerate()` | `mfx_set_framerate()` | ✅ **已实现** | 完整实现（返回 -1，MFX 不支持）

### ✅ 已完成功能

所有核心功能已实现，MFX SDK 已达到 100% 完成度。

**可优化的功能**（非阻塞）:
- `mfx_encode()` - NV12 纹理创建可以进一步优化（缓存纹理描述）

---

## Windows 平台基础设施

### 文件结构

**C++ 文件**：
- `cpp/common/platform/win/win.cpp` - Windows 平台管理（~810行，已不再编译）

**Rust 文件**：
- `src/platform/win/` - 完整的 Rust 实现

### 实现状态 ✅ 100%

| C++ 功能 | Rust 实现 | 状态 |
|---------|----------|------|
| `GetHwcodecGpuSignature()` | `src/platform/win/utils.rs::get_gpu_signature()` | ✅ **已实现** |
| `hwcodec_get_d3d11_texture_width_height()` | `src/platform/win/texture.rs::get_texture_width_height()` | ✅ **已实现** |
| `add_process_to_new_job()` | `src/platform/win/utils.rs::add_process_to_new_job()` | ✅ **已实现** |
| `NativeDevice` 类 | `src/platform/win/device.rs::NativeDevice` | ✅ **已实现** |
| `Adapter` 类 | `src/platform/win/adapter.rs::Adapter` | ✅ **已实现** |
| `Adapters` 类 | `src/platform/win/adapter.rs::Adapters` | ✅ **已实现** |

**注意**：所有 Windows 平台基础设施已完全用 Rust 实现，C++ 代码已不再编译。

---

## 实现进度总结

| SDK | C 包装层 | Rust 业务逻辑 | 总体进度 |
|-----|---------|-------------|---------|
| **AMF** | ✅ 100% | ✅ 100% | **~100%** |
| **NVIDIA** | ✅ 100% | ✅ 100% | **~100%** |
| **MFX** | ✅ 100% | ✅ 100% | **~100%** |
| **Windows 基础设施** | N/A | ✅ 100% | **100%** |

---

## 已修复的差异

根据 `CPP_RUST_DIFFERENCES.md` 文档，以下所有差异已修复：

### ✅ AMF SDK 差异修复

1. **解码器分辨率变化处理** ✅
   - **位置**: `src/vram/amf_rust.rs:1231-1264`
   - **实现**: 检测 `AMF_RESOLUTION_CHANGED` 错误码，调用 `Drain()` → `Terminate()` → 重新 `Init()` → 重新提交输入
   - **C 包装层扩展**: 添加了 `amf_wrapper_component_drain()` 和 `amf_wrapper_create_buffer_from_host()` 函数
   - **错误码支持**: `amf_wrapper_decoder_submit_input()` 现在返回 `2` 表示分辨率变化

2. **Drain 操作** ✅
   - **位置**: `src/vram/amf_rust.rs:1503-1511`
   - **实现**: 在销毁转换器和解码器前调用 `amf_wrapper_component_drain()`

3. **转换器颜色空间属性** ✅
   - **位置**: `src/vram/amf_rust.rs:1348-1377`
   - **实现**: 添加了完整的颜色空间属性设置（INPUT_COLOR_RANGE, OUTPUT_COLOR_RANGE, COLOR_PROFILE, INPUT_TRANSFER_CHARACTERISTIC, INPUT_COLOR_PRIMARIES）

### ✅ NVIDIA SDK 差异修复

1. **解码器重新创建逻辑** ✅
   - **位置**: `src/vram/nv_rust.rs:689-730`
   - **实现**: 当检测到 `-2` 返回值时，自动销毁旧解码器、清理 CUDA 资源、重新创建解码器并重新解码当前帧

---

## 实现优先级

### ✅ 所有高优先级任务已完成

所有核心功能和差异修复已完成，三个 SDK 均已达到 100% 完成度。

### 🔧 可选优化（非阻塞）

1. **`mfx_encode()` - NV12 纹理缓存优化** - 可以进一步优化纹理描述缓存（当前实现已足够）

---

## 文件结构

```
cpp/
├── amf/
│   ├── amf_wrapper.h        ✅ - C 接口头文件
│   ├── amf_wrapper.cpp      ✅ - C 包装层实现
│   ├── amf_encode.cpp       (Windows不编译)
│   └── amf_decode.cpp       (Windows不编译)
├── nv/
│   ├── nv_wrapper.h         ✅ (已完成)
│   ├── nv_wrapper.cpp       ✅ (已完成)
│   ├── nv_encode.cpp        (Windows不编译)
│   └── nv_decode.cpp        (Windows不编译)
└── mfx/
    ├── mfx_wrapper.h        ✅ - C 接口头文件
    ├── mfx_wrapper.cpp      ✅ - C 包装层实现
    ├── mfx_encode.cpp       (Windows不编译)
    └── mfx_decode.cpp       (Windows不编译)

src/vram/
├── amf_rust.rs              ✅ - AMF 业务逻辑实现（100%）
├── nv_rust.rs               ✅ - NVIDIA 业务逻辑实现（100%）
└── mfx_rust.rs              ✅ - MFX 业务逻辑实现（100%）

src/platform/win/
├── device.rs                ✅ - NativeDevice 实现
├── adapter.rs               ✅ - Adapter/Adapters 实现
├── texture.rs               ✅ - 纹理操作实现
└── utils.rs                 ✅ - 工具函数实现
```

---

## 注意事项

1. **文件编码** - 建议使用 UTF-8 编码，避免 C4819 警告
2. **错误处理** - C 包装层返回 0=成功, -1=失败, 1=需要更多输入, 2=分辨率变化（AMF）
3. **资源管理** - 在 Rust 中管理所有资源的生命周期
4. **bindgen 配置** - 需要排除系统类型，避免与 common_ffi.rs 重复定义
5. **编译脚本** - 已创建编译脚本位于 `scripts/` 目录，包括：
   - `compile_all.bat` - 编译所有 C++ 文件
   - `compile_amf_wrapper.bat` - 单独编译 AMF wrapper
   - `compile_mfx_wrapper.bat` - 单独编译 MFX wrapper
   - `compile_nv_wrapper.bat` - 单独编译 NVIDIA wrapper
   - `compile_amf_encode.bat` - 编译 AMF encode
   - `compile_amf_encode_simple.bat` - 简化版 AMF encode 编译

---

**详细差异分析请参考**: `docs/CPP_RUST_DIFFERENCES.md`
