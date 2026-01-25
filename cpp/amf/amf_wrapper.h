#pragma once

// AMF SDK C 包装层头文件
// 只提供 SDK 调用的 C 接口，不包含业务逻辑
// 所有业务逻辑在 Rust 中实现

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// AMF SDK 基础函数
// 初始化 AMF Factory
// 返回: 0=成功, -1=失败
int amf_wrapper_factory_init(void** factory);

// 终止 AMF Factory
void amf_wrapper_factory_terminate(void* factory);

// 创建 AMF Context
// 返回: 0=成功, -1=失败
int amf_wrapper_create_context(void* factory, void** context);

// 初始化 Context 使用 DX11
// 返回: 0=成功, -1=失败
int amf_wrapper_context_init_dx11(void* context, void* device);

// 创建编码器组件
// 返回: 0=成功, -1=失败
int amf_wrapper_create_encoder_component(void* factory, void* context, const char* codec_name, void** component);

// 创建解码器组件
// 返回: 0=成功, -1=失败
int amf_wrapper_create_decoder_component(void* factory, void* context, const char* codec_name, void** component);

// 创建转换器组件
// 返回: 0=成功, -1=失败
int amf_wrapper_create_converter_component(void* factory, void* context, void** component);

// 组件操作
// 设置组件属性
// 返回: 0=成功, -1=失败
int amf_wrapper_component_set_property_int64(void* component, const char* name, int64_t value);
int amf_wrapper_component_set_property_int32(void* component, const char* name, int32_t value);
int amf_wrapper_component_set_property_bool(void* component, const char* name, int32_t value);
int amf_wrapper_component_set_property_double(void* component, const char* name, double value);
int amf_wrapper_component_set_property_wstring(void* component, const char* name, const wchar_t* value);

// 初始化组件
// 返回: 0=成功, -1=失败
int amf_wrapper_component_init(void* component, int32_t format, int32_t width, int32_t height);

// 终止组件
void amf_wrapper_component_terminate(void* component);

// Drain 组件（排空所有待处理的帧）
// 返回: 0=成功, -1=失败
int amf_wrapper_component_drain(void* component);

// Surface 操作
// 从 DX11 纹理创建 Surface
// 返回: 0=成功, -1=失败
int amf_wrapper_create_surface_from_dx11(void* context, void* texture, void** surface);

// 分配 Surface
// 返回: 0=成功, -1=失败
int amf_wrapper_alloc_surface(void* context, int32_t memory_type, int32_t format, int32_t width, int32_t height, void** surface);

// 设置 Surface PTS
void amf_wrapper_surface_set_pts(void* surface, int64_t pts);

// 复制 Surface
// 返回: 0=成功, -1=失败
int amf_wrapper_surface_duplicate(void* surface, int32_t memory_type, void** new_surface);

// 编码器操作
// 提交输入 Surface
// 返回: 0=成功, -1=失败
int amf_wrapper_encoder_submit_input(void* encoder, void* surface);

// 查询输出
// 返回: 0=成功, -1=失败, 1=需要更多输入
int amf_wrapper_encoder_query_output(void* encoder, void** data);

// 解码器操作
// 提交输入数据
// 返回: 0=成功, -1=失败, 2=分辨率变化(AMF_RESOLUTION_CHANGED)
int amf_wrapper_decoder_submit_input(void* decoder, const uint8_t* data, int32_t size, int64_t pts);

// 从主机内存创建 Buffer（用于重新提交输入）
// 返回: 0=成功, -1=失败
int amf_wrapper_create_buffer_from_host(void* context, const uint8_t* data, int32_t size, void** buffer);

// 查询输出 Surface
// 返回: 0=成功, -1=失败, 1=需要更多输入
int amf_wrapper_decoder_query_output(void* decoder, void** surface);

// 转换器操作
// 提交输入
// 返回: 0=成功, -1=失败
int amf_wrapper_converter_submit_input(void* converter, void* surface);

// 查询输出
// 返回: 0=成功, -1=失败, 1=需要更多输入
int amf_wrapper_converter_query_output(void* converter, void** data);

// Buffer 操作
// 获取 Buffer 大小
int32_t amf_wrapper_buffer_get_size(void* buffer);

// 获取 Buffer 数据指针
void* amf_wrapper_buffer_get_native(void* buffer);

// 获取 Buffer 属性（int64）
// 返回: 0=成功, -1=失败
int amf_wrapper_buffer_get_property_int64(void* buffer, const char* name, int64_t* value);

// Surface 操作
// 获取 Surface 格式
int32_t amf_wrapper_surface_get_format(void* surface);

// 获取 Surface 宽度
int32_t amf_wrapper_surface_get_width(void* surface);

// 获取 Surface 高度
int32_t amf_wrapper_surface_get_height(void* surface);

// 获取 Surface 平面数
int32_t amf_wrapper_surface_get_planes_count(void* surface);

// 获取 Surface 平面
void* amf_wrapper_surface_get_plane_at(void* surface, int32_t plane_index);

// 获取平面数据指针
void* amf_wrapper_plane_get_native(void* plane);

// 释放资源
void amf_wrapper_release(void* ptr);

#ifdef __cplusplus
}
#endif
