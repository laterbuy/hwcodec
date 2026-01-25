# Windows 平台代码分析文档

## 目录结构

```
cpp/common/platform/win/
├── win.h              # Windows 平台核心头文件
├── win.cpp            # Windows 平台核心实现
├── bmp.cpp            # BMP 文件处理
├── dump.cpp           # 纹理转储功能
├── vertex_shader.h    # 顶点着色器（编译后的字节码）
└── pixel_shader_601.h # 像素着色器（NV12 到 BGRA 转换，编译后的字节码）
```

## 核心功能概述

`win` 目录提供了 Windows 平台下的 DirectX 11 (D3D11) 硬件加速视频编解码支持，主要包括：

1. **D3D11 设备管理**：创建和管理 DirectX 11 设备、上下文
2. **纹理管理**：创建、共享和管理 D3D11 纹理
3. **视频处理**：使用 D3D11 Video Processor 进行视频格式转换和处理
4. **格式转换**：NV12 ↔ BGRA 格式转换（使用 GPU 着色器）
5. **调试工具**：纹理转储、BMP 保存等功能

---

## 文件详细分析

### 1. win.h / win.cpp

#### 1.1 NativeDevice 类

**功能**：管理 D3D11 设备和视频处理功能的核心类

**主要成员变量**：
- `device_`：D3D11 设备
- `context_`：D3D11 设备上下文
- `video_device_`：D3D11 视频设备
- `video_context_` / `video_context1_`：D3D11 视频上下文
- `video_processor_`：视频处理器
- `texture_`：纹理池（支持多纹理缓冲）

**主要方法**：

##### 初始化相关
- `Init(int64_t luid, ID3D11Device *device, int pool_size)`
  - 初始化设备，可通过 LUID 或现有设备初始化
  - 设置多线程保护
  - 初始化查询对象和视频设备
  - 创建纹理池

- `InitFromLuid(int64_t luid)`
  - 通过适配器 LUID 创建 D3D11 设备
  - 使用 `D3D11_CREATE_DEVICE_VIDEO_SUPPORT` 和 `D3D11_CREATE_DEVICE_BGRA_SUPPORT` 标志

- `InitFromDevice(ID3D11Device *device)`
  - 从现有 D3D11 设备初始化
  - 获取适配器和工厂对象

##### 纹理管理
- `EnsureTexture(int width, int height)`
  - 确保存在指定尺寸的共享纹理（BGRA 格式）
  - 纹理标志：`D3D11_RESOURCE_MISC_SHARED`（支持跨进程共享）

- `SetTexture(ID3D11Texture2D *texture)`
  - 设置当前使用的纹理

- `GetSharedHandle()`
  - 获取纹理的共享句柄（用于跨进程共享）

- `GetCurrentTexture()`
  - 获取当前纹理

- `next()`
  - 切换到下一个纹理（循环使用纹理池）

##### 视频处理
- `Process(ID3D11Texture2D *in, ID3D11Texture2D *out, ...)`
  - 使用 D3D11 Video Processor 进行视频处理
  - 支持颜色空间转换（`DXGI_COLOR_SPACE_TYPE`）
  - 支持裁剪和缩放

- `BgraToNv12(...)`
  - 将 BGRA 纹理转换为 NV12 格式
  - 使用 D3D11 Video Processor 进行转换

- `Nv12ToBgra(...)`
  - 将 NV12 纹理转换为 BGRA 格式
  - 使用 GPU 着色器进行转换（硬件加速）

##### NV12 到 BGRA 转换（着色器实现）
- `nv12_to_bgra_set_srv()`：设置着色器资源视图（SRV）
- `nv12_to_bgra_set_rtv()`：设置渲染目标视图（RTV）
- `nv12_to_bgra_set_view_port()`：设置视口
- `nv12_to_bgra_set_sample()`：设置采样器
- `nv12_to_bgra_set_shader()`：设置顶点和像素着色器
- `nv12_to_bgra_set_vertex_buffer()`：设置顶点缓冲区
- `nv12_to_bgra_draw()`：执行绘制

##### 查询和同步
- `BeginQuery()` / `EndQuery()` / `Query()`
  - 使用 D3D11 查询对象进行 GPU 同步
  - 确保 GPU 操作完成

##### 硬件检测
- `GetVendor()`
  - 获取 GPU 厂商（NVIDIA/AMD/Intel）

- `support_decode(DataFormat format)`
  - 检查是否支持硬件解码（H264/H265）
  - 检测混合解码（hybrid decode）并避免使用

- `isFormatHybridDecodedByHardware(...)`
  - 检测特定 GPU 是否使用混合解码
  - 针对 Intel 和 NVIDIA 的特定 GPU 型号进行黑名单检查
  - 避免性能较差的混合解码

#### 1.2 Adapter 类

**功能**：管理单个 GPU 适配器

**主要方法**：
- `Init(IDXGIAdapter1 *adapter1)`
  - 初始化适配器
  - 创建 D3D11 设备
  - 对于 Intel GPU，启用多线程保护

#### 1.3 Adapters 类

**功能**：管理多个 GPU 适配器

**主要方法**：
- `Init(AdapterVendor vendor)`
  - 枚举并初始化指定厂商的所有适配器

- `GetFirstAdapterIndex(AdapterVendor vendor)`
  - 获取指定厂商的第一个适配器索引

#### 1.4 工具函数

- `GetHwcodecGpuSignature()`
  - 计算 GPU 签名（用于检测 GPU 驱动更新）
  - 基于 VendorId、DeviceId、SubSysId、Revision 和 UMD 版本

- `hwcodec_get_d3d11_texture_width_height()`
  - 获取 D3D11 纹理的宽度和高度

- `add_process_to_new_job(DWORD process_id)`
  - 将进程添加到新的作业对象
  - 用于进程管理（当作业对象关闭时自动终止子进程）

---

### 2. bmp.cpp

**功能**：将 BGRA 纹理保存为 BMP 文件

**主要函数**：

- `CreateBmpFile(LPCWSTR wszBmpFile, BYTE *pData, ...)`
  - 创建 24 位 BMP 文件
  - 处理 BMP 文件头（54 字节）
  - 处理图像数据（BMP 格式要求从下到上存储）

- `createBgraBmpFile(ID3D11Device *device, ID3D11Texture2D *texture, ...)`
  - 从 D3D11 纹理创建 BMP 文件
  - 创建 staging 纹理（CPU 可读）
  - 复制纹理数据到 staging 纹理
  - 映射到 CPU 内存
  - 转换 BGRA 到 BGR（去除 Alpha 通道）
  - 调用 `CreateBmpFile` 保存

- `SaveBgraBmps(ID3D11Device *device, void *texture, int cycle)`
  - 周期性保存 BGRA 纹理为 BMP 文件
  - 按指定周期（cycle）保存
  - 文件名包含索引和时间戳

**使用场景**：
- 调试视频编码/解码
- 验证纹理内容
- 性能分析

---

### 3. dump.cpp

**功能**：将 NV12 格式纹理转储到文件

**主要函数**：

- `dumpTexture(ID3D11Device *device, ID3D11Texture2D *texture, ...)`
  - 将 NV12 纹理保存为原始数据文件
  - 创建 staging 纹理
  - 映射到 CPU 内存
  - 分别保存 Y 平面和 UV 平面（交错存储）
  - 支持裁剪（cropW, cropH）
  - 支持 P010 格式（10 位）

**文件格式**：
- Y 平面：宽度 × 高度 字节
- UV 平面：宽度 × (高度/2) 字节（交错存储）

**使用场景**：
- 调试视频解码
- 验证 NV12 数据
- 性能分析

---

### 4. vertex_shader.h

**功能**：顶点着色器字节码

**说明**：
- 编译后的 HLSL 着色器字节码
- 用于 NV12 到 BGRA 转换的渲染管线
- 简单的传递着色器（pass-through）
- 输入：位置（POSITION）和纹理坐标（TEXCOORD）
- 输出：位置和纹理坐标

**着色器代码**（原始 HLSL）：
```hlsl
struct VS_INPUT {
    float4 Pos : POSITION;
    float2 Tex : TEXCOORD;
};

struct VS_OUTPUT {
    float4 Pos : SV_POSITION;
    float2 Tex : TEXCOORD;
};

VS_OUTPUT VS(VS_INPUT input) {
    return input;
}
```

---

### 5. pixel_shader_601.h

**功能**：像素着色器字节码（用于 NV12 到 BGRA 转换）

**说明**：
- 编译后的 HLSL 着色器字节码
- 实现 Rec. 601 标准的 YUV 到 RGB 转换
- 输入：两个纹理（Y 平面和 UV 平面）
- 输出：BGRA 颜色

**转换公式**（Rec. 601）：
```
Y' = Y - 0.0625
U' = U - 0.5
V' = V - 0.5

R = saturate(Y' * 1.164384 + V' * 1.596027)
G = saturate(Y' * 1.164384 - U' * 0.391762 - V' * 0.812968)
B = saturate(Y' * 1.164384 + U' * 2.017232)
```

**着色器代码**（原始 HLSL）：
```hlsl
Texture2D g_txFrame0 : register(t0);  // Y plane
Texture2D g_txFrame1 : register(t1);  // UV plane
SamplerState g_Sam : register(s0);

float4 PS(PS_INPUT input) : SV_TARGET {
    float y = g_txFrame0.Sample(g_Sam, input.Tex).r;
    y = 1.16438356164383561643836 * (y - 0.0625);
    
    float2 uv = g_txFrame1.Sample(g_Sam, input.Tex).rg - float2(0.5f, 0.5f);
    float u = uv.x;
    float v = uv.y;
    
    float r = saturate(y + 1.596026785714285714285714286 * v);
    float g = saturate(y - 0.8129676472377777777777777771 * v - 0.391762290094914914914914 * u);
    float b = saturate(y + 2.017232142857142857142857142 * u);
    
    return float4(r, g, b, 1.0f);
}
```

---

## 技术细节

### 1. 多线程保护

对于 Intel GPU，代码会启用多线程保护：
```cpp
ComPtr<ID3D10Multithread> hmt;
context_.As(&hmt);
hmt->SetMultithreadProtected(TRUE);
```

这允许从多个线程安全地访问 D3D11 上下文。

### 2. 纹理共享

使用 `D3D11_RESOURCE_MISC_SHARED` 标志创建共享纹理：
- 可以通过 `GetSharedHandle()` 获取共享句柄
- 其他进程可以通过 `OpenSharedResource()` 访问纹理
- 用于跨进程视频数据传输

### 3. 视频处理器

使用 D3D11 Video Processor 进行硬件加速的视频处理：
- 颜色空间转换
- 缩放和裁剪
- 格式转换（BGRA ↔ NV12）

### 4. 混合解码检测

代码会检测并避免使用混合解码（hybrid decode）：
- **Intel**：某些旧型号（Haswell、Broadwell 等）对 HEVC 使用混合解码
- **NVIDIA**：Feature Set E 的 GPU（Kepler、Maxwell 等）对 HEVC 使用混合解码
- 混合解码性能较差，应避免使用

### 5. GPU 签名计算

用于检测 GPU 驱动更新：
```cpp
signature += desc.VendorId;
signature += desc.DeviceId;
signature += desc.SubSysId;
signature += desc.Revision;
signature += umd_version.QuadPart;  // UMD 版本
```

---

## 依赖关系

### DirectX 依赖
- `d3d11.h` / `d3d11_1.h`：D3D11 API
- `dxgi.h`：DXGI API（适配器枚举）
- `DirectXMath.h`：数学库
- `directxcolors.h`：颜色常量

### 链接库
- `d3d11.lib`：D3D11 库
- `dxgi.lib`：DXGI 库

---

## 使用示例

### 初始化设备
```cpp
NativeDevice device;
int64_t luid = ...;  // 从适配器获取
if (!device.Init(luid, nullptr, 3)) {
    // 错误处理
}
```

### 格式转换
```cpp
// NV12 到 BGRA
ID3D11Texture2D *nv12Texture = ...;
ID3D11Texture2D *bgraTexture = ...;
device.Nv12ToBgra(width, height, nv12Texture, bgraTexture, 0);

// BGRA 到 NV12
device.BgraToNv12(bgraTexture, nv12Texture, width, height, 
                  DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709,
                  DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709);
```

### 保存纹理为 BMP
```cpp
SaveBgraBmps(device.device_.Get(), bgraTexture, 30);  // 每 30 帧保存一次
```

---

## 注意事项

1. **线程安全**：D3D11 上下文默认不是线程安全的，需要启用多线程保护（特别是 Intel GPU）

2. **资源管理**：使用 `ComPtr` 自动管理 COM 对象生命周期

3. **错误处理**：使用 `HRB` 宏进行错误检查（失败时返回 false）

4. **性能优化**：
   - 纹理池减少分配开销
   - 着色器状态缓存（只在尺寸变化时重新设置）
   - 使用硬件加速的视频处理器

5. **兼容性**：
   - 需要 DirectX 11.0 支持
   - 需要支持视频处理的 GPU
   - 某些旧 GPU 可能不支持某些格式

---

## 相关参考

- [D3D11 Video Processing](https://docs.microsoft.com/en-us/windows/win32/medfound/direct3d-11-video-apis)
- [DXGI Color Space](https://docs.microsoft.com/en-us/windows/win32/api/dxgicommon/ne-dxgicommon-dxgi_color_space_type)
- [Rec. 601 Color Space](https://en.wikipedia.org/wiki/Rec._601)
