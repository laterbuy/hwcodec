# AMF HEVC 编码器实现参考

## 与 rustdesk-org/hwcodec 对比

参考实现：[rustdesk-org/hwcodec cpp/amf](https://github.com/rustdesk-org/hwcodec/tree/master/cpp/amf)

本地为单文件桥接 `cpp/amf_bridge.cpp`，rustdesk 仓库为 `cpp/amf/` 目录下多文件。建议本地对比方式：

```bash
# 克隆参考仓库（仅用于对比）
git clone --depth 1 https://github.com/rustdesk-org/hwcodec.git /tmp/hwcodec-ref
# 对比 AMF 编码器创建与初始化
diff -u /tmp/hwcodec-ref/cpp/amf/ cpp/amf_bridge.cpp
# 或仅查看其 HEVC 相关实现
ls /tmp/hwcodec-ref/cpp/amf/
```

关注点：

- **HEVC 编码器**：`CreateComponent(AMFVideoEncoder_HEVC)` 后的属性设置顺序与必填项（USAGE、PROFILE、FRAMESIZE、FRAMERATE、TARGET_BITRATE、GOP_SIZE、MEMORY_TYPE 等）。
- **Init 前属性**：AMF 要求 USAGE 等静态属性在 `Init()` 前设置；是否设置 QUALITY_PRESET、RATE_CONTROL_METHOD 等。
- **失败处理**：Init 失败后是否调用 Release、是否重试其他 USAGE。

## 本仓库当前实现要点（cpp/amf_bridge.cpp）

- **HEVC 已知问题**：在部分环境（驱动自带的 amfrt64.dll）下，对 HEVC 组件调用 **SetProperty** 会触发 **STATUS_ACCESS_VIOLATION**（无论 C++ 虚表调用还是通过 pVtbl 调用）。当前实现**不设置任何属性**直接 `Init()`，因缺少 USAGE 等必填项 Init 返回 AMF_FAIL，编码器创建失败但**不崩溃**。
- **结果**：AMF 的 H.265 编码器在本库中实际不可用；`color_to_h265` 若仅检测到 AMF/H265 会报错，建议使用 **MFX** 或 **NV** 的 H.265 编码器。
- **参考实现**：`cpp_old/amf/amf_encode.cpp` 中 SetParams(AMFVideoEncoder_HEVC) 使用 USAGE=LOW_LATENCY_HIGH_QUALITY、LOWLATENCY_MODE、QUALITY_PRESET、RATE_CONTROL_METHOD 等；因上述 SetProperty 崩溃，当前未采用。
- **输入格式**：AMF_SURFACE_BGRA，与 AVC 一致。

## 官方文档

- [AMF Video Encode HEVC API](https://github.com/GPUOpen-LibrariesAndSDKs/AMF/blob/master/amf/doc/AMF_Video_Encode_HEVC_API.pdf)
- [Guide for Video CODEC Encoder App Developers](https://github.com/GPUOpen-LibrariesAndSDKs/AMF/wiki/Guide%20for%20Video%20CODEC%20Encoder%20App%20Developers)
- AMF C++ 示例：GPUOpen-LibrariesAndSDKs/AMF 的 `amf/public/samples/CPPSamples`（本仓库 externals 未包含，可单独下载 AMF SDK 查看）。
