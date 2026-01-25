#ifndef WIN_RUST_FFI_H
#define WIN_RUST_FFI_H

#include <windows.h>
#include <d3d11.h>
#include <d3d11_1.h>
#include <dxgi.h>
#include <dxgi1_2.h>
#include "../../common.h"

#ifdef __cplusplus
extern "C" {
#endif

// LUID 宏定义（从 win.h 移过来，因为 C++ 代码仍在使用）
#define LUID(desc) \
  (((int64_t)(desc).AdapterLuid.HighPart << 32) | (desc).AdapterLuid.LowPart)

// ============================================================================
// NativeDevice FFI 接口
// ============================================================================

// 不透明的 NativeDevice 句柄
typedef void* NativeDeviceHandle;

// 创建新的 NativeDevice
// luid: 适配器 LUID，如果为 0 则使用默认适配器
// device: 可选的现有 D3D11 设备指针，如果为 nullptr 则从 LUID 创建
// pool_size: 纹理池大小
// 返回: NativeDevice 句柄，失败返回 nullptr
NativeDeviceHandle hwcodec_native_device_new(int64_t luid, ID3D11Device* device, int pool_size);

// 释放 NativeDevice
void hwcodec_native_device_destroy(NativeDeviceHandle handle);

// 初始化纹理池
int hwcodec_native_device_ensure_texture(NativeDeviceHandle handle, unsigned int width, unsigned int height);

// 设置纹理
void hwcodec_native_device_set_texture(NativeDeviceHandle handle, ID3D11Texture2D* texture);

// 获取共享句柄
HANDLE hwcodec_native_device_get_shared_handle(NativeDeviceHandle handle);

// 获取当前纹理
ID3D11Texture2D* hwcodec_native_device_get_current_texture(NativeDeviceHandle handle);

// 移动到下一个纹理
int hwcodec_native_device_next(NativeDeviceHandle handle);

// 开始查询
void hwcodec_native_device_begin_query(NativeDeviceHandle handle);

// 结束查询
void hwcodec_native_device_end_query(NativeDeviceHandle handle);

// 查询完成状态
int hwcodec_native_device_query(NativeDeviceHandle handle);

// 获取设备指针
ID3D11Device* hwcodec_native_device_get_device(NativeDeviceHandle handle);

// 获取上下文指针
ID3D11DeviceContext* hwcodec_native_device_get_context(NativeDeviceHandle handle);

// 获取视频设备指针
ID3D11VideoDevice* hwcodec_native_device_get_video_device(NativeDeviceHandle handle);

// 获取视频上下文指针
ID3D11VideoContext* hwcodec_native_device_get_video_context(NativeDeviceHandle handle);

// 获取视频上下文1指针
ID3D11VideoContext1* hwcodec_native_device_get_video_context1(NativeDeviceHandle handle);

// 获取厂商
int hwcodec_native_device_get_vendor(NativeDeviceHandle handle);

// 检查是否支持硬件解码
int hwcodec_native_device_support_decode(NativeDeviceHandle handle, int format);

// 视频处理
int hwcodec_native_device_process(
    NativeDeviceHandle handle,
    ID3D11Texture2D* input,
    ID3D11Texture2D* output,
    unsigned int width,
    unsigned int height,
    const D3D11_VIDEO_PROCESSOR_CONTENT_DESC* content_desc,
    DXGI_COLOR_SPACE_TYPE color_space_in,
    DXGI_COLOR_SPACE_TYPE color_space_out,
    unsigned int array_slice);

// 将 BGRA 纹理转换为 NV12
int hwcodec_native_device_bgra_to_nv12(
    NativeDeviceHandle handle,
    ID3D11Texture2D* bgra_texture,
    ID3D11Texture2D* nv12_texture,
    unsigned int width,
    unsigned int height,
    DXGI_COLOR_SPACE_TYPE color_space_in,
    DXGI_COLOR_SPACE_TYPE color_space_out);

// 将 NV12 纹理转换为 BGRA
int hwcodec_native_device_nv12_to_bgra(
    NativeDeviceHandle handle,
    ID3D11Texture2D* nv12_texture,
    ID3D11Texture2D* bgra_texture,
    unsigned int width,
    unsigned int height,
    unsigned int nv12_array_index);

// ============================================================================
// Adapters FFI 接口
// ============================================================================

// 不透明的 Adapters 句柄
typedef void* AdaptersHandle;

// 创建新的 Adapters
AdaptersHandle hwcodec_adapters_new(int vendor);

// 释放 Adapters
void hwcodec_adapters_destroy(AdaptersHandle handle);

// 获取第一个适配器索引
int hwcodec_adapters_get_first_adapter_index(int vendor);

// 获取适配器数量
int hwcodec_adapters_get_count(AdaptersHandle handle);

// 获取指定索引的适配器设备
ID3D11Device* hwcodec_adapters_get_adapter_device(AdaptersHandle handle, int index);

// 获取指定索引的适配器描述
int hwcodec_adapters_get_adapter_desc(AdaptersHandle handle, int index, DXGI_ADAPTER_DESC1* desc);

// 获取指定索引的适配器 LUID
int64_t hwcodec_adapters_get_adapter_luid(AdaptersHandle handle, int index);

// ============================================================================
// BMP 和 Dump FFI 接口
// ============================================================================

// 周期性保存 BGRA 纹理为 BMP 文件
void SaveBgraBmps(ID3D11Device* device, void* texture, int cycle);

// 转储 NV12 纹理到文件
int dumpTexture(ID3D11Device* device, ID3D11Texture2D* texture, int cropW, int cropH, const char* filename);

#ifdef __cplusplus
}
#endif

#endif // WIN_RUST_FFI_H
