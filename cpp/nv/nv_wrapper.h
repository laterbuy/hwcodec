#pragma once

// NVIDIA Video Codec SDK C 包装层头文件
// 只提供 SDK 调用的 C 接口，不包含业务逻辑
// 所有业务逻辑在 Rust 中实现

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// 编码器操作
// ============================================================================

// 创建编码器
// 返回: 0=成功, -1=失败
int nv_wrapper_create_encoder(
    void* cuda_dl,
    void* nvenc_dl,
    void* device,
    int32_t width,
    int32_t height,
    int32_t codec_id,  // 0=H264, 1=H265
    int32_t bitrate_kbps,
    int32_t framerate,
    int32_t gop,
    void** encoder
);

// 销毁编码器
void nv_wrapper_destroy_encoder(void* encoder);

// 获取下一个输入帧
// 返回: 输入帧指针，失败返回 NULL
void* nv_wrapper_encoder_get_next_input_frame(void* encoder);

// 编码一帧
// packet_data: 输出数据缓冲区
// packet_size: 输入为缓冲区大小，输出为实际数据大小
// picture_type: 输出图片类型（0=I, 1=IDR, 2=P, 3=B）
// 返回: 0=成功, -1=失败
int nv_wrapper_encoder_encode_frame(
    void* encoder,
    void* input_texture,
    int64_t timestamp,
    void* packet_data,
    uint32_t* packet_size,
    uint32_t* picture_type
);

// 重新配置编码器（用于动态改变码率/帧率）
// 返回: 0=成功, -1=失败
int nv_wrapper_encoder_reconfigure(
    void* encoder,
    int32_t bitrate_kbps,
    int32_t framerate
);

// ============================================================================
// 解码器操作
// ============================================================================

// 创建解码器
// 返回: 0=成功, -1=失败
int nv_wrapper_create_decoder(
    void* cuda_dl,
    void* cuvid_dl,
    void* cu_context,
    int32_t codec_id,  // 0=H264, 1=H265
    void** decoder
);

// 销毁解码器
void nv_wrapper_destroy_decoder(void* decoder);

// 解码数据
// 返回: 解码的帧数，失败返回 -1，需要重新创建返回 -2
int nv_wrapper_decoder_decode(
    void* decoder,
    const uint8_t* data,
    int32_t length,
    uint32_t flags
);

// 获取解码后的帧数据
// 返回: 帧数据指针（CUDA 设备内存），失败返回 NULL
void* nv_wrapper_decoder_get_frame(void* decoder);

// 获取解码器宽度
int32_t nv_wrapper_decoder_get_width(void* decoder);

// 获取解码器高度
int32_t nv_wrapper_decoder_get_height(void* decoder);

// 获取解码器色度高度
int32_t nv_wrapper_decoder_get_chroma_height(void* decoder);

// ============================================================================
// CUDA 操作
// ============================================================================

// 加载 CUDA 和 NVENC 库
// 返回: 0=成功, -1=失败
int nv_wrapper_load_encoder_driver(void** cuda_dl, void** nvenc_dl);

// 释放 CUDA 和 NVENC 库
void nv_wrapper_free_encoder_driver(void** cuda_dl, void** nvenc_dl);

// 加载 CUDA 和 NVDEC 库
// 返回: 0=成功, -1=失败
int nv_wrapper_load_decoder_driver(void** cuda_dl, void** cuvid_dl);

// 释放 CUDA 和 NVDEC 库
void nv_wrapper_free_decoder_driver(void** cuda_dl, void** cuvid_dl);

// 初始化 CUDA
// 返回: 0=成功, -1=失败
int nv_wrapper_cuda_init(void* cuda_dl);

// 从 D3D11 适配器获取 CUDA 设备
// 返回: 0=成功, -1=失败
int nv_wrapper_cuda_get_device_from_d3d11(void* cuda_dl, void* adapter, uint32_t* cu_device);

// 创建 CUDA 上下文
// 返回: 0=成功, -1=失败
int nv_wrapper_cuda_create_context(void* cuda_dl, uint32_t cu_device, void** cu_context);

// 销毁 CUDA 上下文
void nv_wrapper_cuda_destroy_context(void* cuda_dl, void* cu_context);

// 推送 CUDA 上下文
// 返回: 0=成功, -1=失败
int nv_wrapper_cuda_push_context(void* cuda_dl, void* cu_context);

// 弹出 CUDA 上下文
// 返回: 0=成功, -1=失败
int nv_wrapper_cuda_pop_context(void* cuda_dl);

// ============================================================================
// 纹理操作（用于解码器）
// ============================================================================

// 注册 D3D11 纹理为 CUDA 资源
// 返回: 0=成功, -1=失败
int nv_wrapper_cuda_register_texture(void* cuda_dl, void* texture, void** cu_resource);

// 注销 CUDA 资源
void nv_wrapper_cuda_unregister_texture(void* cuda_dl, void* cu_resource);

// 映射 CUDA 资源
// 返回: 0=成功, -1=失败
int nv_wrapper_cuda_map_resource(void* cuda_dl, void* cu_resource);

// 取消映射 CUDA 资源
// 返回: 0=成功, -1=失败
int nv_wrapper_cuda_unmap_resource(void* cuda_dl, void* cu_resource);

// 获取映射的 CUDA 数组
// 返回: CUDA 数组指针，失败返回 NULL
void* nv_wrapper_cuda_get_mapped_array(void* cuda_dl, void* cu_resource);

// 复制 CUDA 内存（设备到数组）
// 返回: 0=成功, -1=失败
int nv_wrapper_cuda_memcpy_device_to_array(
    void* cuda_dl,
    void* dst_array,
    const void* src_device,
    uint32_t width,
    uint32_t height,
    uint32_t src_pitch
);

#ifdef __cplusplus
}
#endif
