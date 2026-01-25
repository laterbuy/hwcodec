#pragma once

// Intel MediaSDK (MFX) C 包装层头文件
// 只提供 SDK 调用的 C 接口，不包含业务逻辑
// 所有业务逻辑在 Rust 中实现

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// MFX Session 操作
// 初始化 MFX Session
// 返回: 0=成功, -1=失败
int mfx_wrapper_session_init(void** session);

// 设置 D3D11 设备句柄
// 返回: 0=成功, -1=失败
int mfx_wrapper_session_set_handle_d3d11(void* session, void* device);

// 设置帧分配器
// 返回: 0=成功, -1=失败
int mfx_wrapper_session_set_frame_allocator(void* session, void* allocator);

// 关闭 Session
void mfx_wrapper_session_close(void* session);

// 编码器操作
// 创建编码器
// 返回: 0=成功, -1=失败
int mfx_wrapper_create_encoder(void* session, void** encoder);

// 关闭编码器
void mfx_wrapper_encoder_close(void* encoder);

// 获取视频参数
// 返回: 0=成功, -1=失败
int mfx_wrapper_encoder_get_video_param(void* encoder, void* params);

// 重置编码器参数
// 返回: 0=成功, -1=失败
int mfx_wrapper_encoder_reset(void* encoder, void* params);

// 提交输入 Surface
// 返回: 0=成功, 1=需要更多输入, -1=失败
int mfx_wrapper_encoder_encode_frame_async(void* encoder, void* surface, void* bitstream, void* syncp);

// 同步操作
// 返回: 0=成功, -1=失败
int mfx_wrapper_sync_operation(void* session, void* syncp, uint32_t timeout);

// 解码器操作
// 创建解码器
// 返回: 0=成功, -1=失败
int mfx_wrapper_create_decoder(void* session, void** decoder);

// 关闭解码器
void mfx_wrapper_decoder_close(void* decoder);

// 查询解码器能力
// 返回: 0=成功, -1=失败
int mfx_wrapper_decoder_query(void* decoder, void* params, void* caps);

// 初始化解码器
// 返回: 0=成功, -1=失败
int mfx_wrapper_decoder_init(void* decoder, void* params);

// 解码一帧
// 返回: 0=成功, 1=需要更多数据, -1=失败
int mfx_wrapper_decoder_decode_frame_async(void* decoder, void* bitstream, void* surface_work, void* surface_out, void* syncp);

// 获取解码输出 Surface
// 返回: 0=成功, 1=需要更多 Surface, -1=失败
int mfx_wrapper_decoder_get_surface(void* decoder, void** surface);

// Surface 操作
// 获取 Surface 的 MemId (D3D11 纹理指针)
void* mfx_wrapper_surface_get_mem_id(void* surface);

// 获取 Surface 信息
// 返回: 0=成功, -1=失败
int mfx_wrapper_surface_get_info(void* surface, void* info);

// Bitstream 操作
// 初始化 Bitstream
void mfx_wrapper_bitstream_init(void* bitstream, void* data, uint32_t length);

// 获取 Bitstream 数据
void* mfx_wrapper_bitstream_get_data(void* bitstream);

// 获取 Bitstream 长度
uint32_t mfx_wrapper_bitstream_get_length(void* bitstream);

// 获取 Bitstream 帧类型
uint32_t mfx_wrapper_bitstream_get_frame_type(void* bitstream);

// 帧分配器操作（简化接口，实际使用 D3D11 分配器）
// 创建 D3D11 帧分配器
// 返回: 0=成功, -1=失败
int mfx_wrapper_create_d3d11_frame_allocator(void* device, void** allocator);

// 分配帧
// 返回: 0=成功, -1=失败
int mfx_wrapper_allocator_alloc(void* allocator, void* request, void* response);

// 释放帧
// 返回: 0=成功, -1=失败
int mfx_wrapper_allocator_free(void* allocator, void* response);

// 释放分配器
void mfx_wrapper_allocator_release(void* allocator);

// 高级接口：编码器参数设置（封装 mfxVideoParam 操作）
// 创建并初始化编码器参数结构
// 返回: 参数结构指针，失败返回 NULL
void* mfx_wrapper_create_encoder_params(
    int32_t codec_id,      // MFX_CODEC_AVC 或 MFX_CODEC_HEVC
    int32_t width,
    int32_t height,
    int32_t framerate,
    int32_t bitrate_kbps,
    int32_t gop
);

// 释放编码器参数结构
void mfx_wrapper_destroy_encoder_params(void* params);

// 高级接口：查询编码器并分配 Surface
// 返回: Surface 数量，失败返回 -1
int32_t mfx_wrapper_encoder_query_and_alloc_surfaces(
    void* encoder,
    void* params,
    void** surfaces,  // 输出：Surface 数组指针
    int32_t* surface_count  // 输出：Surface 数量
);

// 高级接口：初始化解码器参数（从 Bitstream 头）
// 返回: 0=成功, -1=失败
int mfx_wrapper_decoder_init_from_bitstream(
    void* decoder,
    void* bitstream,
    void* params
);

// 高级接口：查询解码器并分配 Surface
// 返回: Surface 数量，失败返回 -1
int32_t mfx_wrapper_decoder_query_and_alloc_surfaces(
    void* decoder,
    void* params,
    void** surfaces,  // 输出：Surface 数组指针
    int32_t* surface_count  // 输出：Surface 数量
);

// 辅助函数：获取空闲 Surface 索引
// 返回: Surface 索引，失败返回 -1
int32_t mfx_wrapper_get_free_surface_index(
    void* surfaces,  // Surface 数组
    int32_t surface_count
);

// 辅助函数：对齐宽度（16字节对齐）
int32_t mfx_wrapper_align16(int32_t value);

// 辅助函数：对齐高度（16字节对齐，渐进式）
int32_t mfx_wrapper_align16_height(int32_t value);

// 高级接口：查询编码器并初始化
// 返回: 0=成功, -1=失败
int mfx_wrapper_encoder_query_and_init(
    void* encoder,
    void* params
);

// 高级接口：查询编码器 Surface 需求
// 返回: Surface 数量，失败返回 -1
int32_t mfx_wrapper_encoder_query_iosurf(
    void* encoder,
    void* params
);

// 高级接口：创建 Bitstream 结构
// 返回: Bitstream 指针，失败返回 NULL
void* mfx_wrapper_create_bitstream(uint32_t max_length);

// 释放 Bitstream 结构
void mfx_wrapper_destroy_bitstream(void* bitstream);

// 高级接口：创建 Surface 数组
// 返回: Surface 数组指针，失败返回 NULL
void* mfx_wrapper_create_surface_array(
    int32_t count,
    void* frame_info  // mfxFrameInfo*
);

// 释放 Surface 数组
void mfx_wrapper_destroy_surface_array(void* surfaces);

// 高级接口：设置 Surface 的 MemId
void mfx_wrapper_surface_set_mem_id(void* surface, void* mem_id);

// 高级接口：创建 SyncPoint
void* mfx_wrapper_create_syncpoint();

// 释放 SyncPoint
void mfx_wrapper_destroy_syncpoint(void* syncp);

// 高级接口：获取 Surface 数组中的指定 Surface
void* mfx_wrapper_get_surface_at(void* surfaces, int32_t index);

// 高级接口：解码器从 Bitstream 头初始化解码参数
// 返回: 0=成功, -1=失败
int mfx_wrapper_decoder_decode_header(
    void* decoder,
    void* bitstream,
    void* params
);

// 高级接口：查询解码器 Surface 需求
// 返回: Surface 数量，失败返回 -1
int32_t mfx_wrapper_decoder_query_iosurf(
    void* decoder,
    void* params
);

// 高级接口：设置解码器参数（基础参数）
void* mfx_wrapper_create_decoder_params(
    int32_t codec_id
);

// 释放解码器参数结构
void mfx_wrapper_destroy_decoder_params(void* params);

// 高级接口：初始化解码器（从 Bitstream 头，分配 Surface）
// 返回: 0=成功, -1=失败
int mfx_wrapper_decoder_initialize_from_bitstream(
    void* decoder,
    void* bitstream,
    void* params,
    void* allocator,
    void** surfaces,  // 输出：Surface 数组指针
    int32_t* surface_count  // 输出：Surface 数量
);

// 高级接口：设置 Surface 的 MemId（从分配器响应）
void mfx_wrapper_surface_set_mem_id_from_response(
    void* surface,
    void* response,  // mfxFrameAllocResponse*
    int32_t index
);

#ifdef __cplusplus
}
#endif
