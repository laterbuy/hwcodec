#pragma once

/* 避免 /FI 下先处理本头文件时找不到 <cstdint>：MSVC 下用内置类型 */
#if defined(_MSC_VER)
typedef unsigned __int8 uint8_t;
typedef __int32  int32_t;
typedef __int64  int64_t;
#else
#include <cstdint>
#endif

struct AmfEncoder { void* impl; };
struct AmfDecoder { void* impl; };
struct EncodedFrame;
struct DecodedFrame;

bool amf_IsDriverAvailable();
/** True when AMF decode is available (HWCODEC_AMF_FULL build). */
bool amf_IsDecodeImplemented();

extern "C++" {
    AmfEncoder* amf_CreateEncoder(uint8_t* device, int32_t width, int32_t height, int32_t codec_id, int32_t bitrate, int32_t framerate, int32_t gop);
    EncodedFrame* amf_EncodeFrame(AmfEncoder* encoder, uint8_t* texture, int64_t timestamp);
    void amf_DestroyEncoder(AmfEncoder* encoder);
    void amf_SetBitrate(AmfEncoder* encoder, int32_t bitrate);
    void amf_SetFramerate(AmfEncoder* encoder, int32_t framerate);

    AmfDecoder* amf_CreateDecoder(uint8_t* device, int32_t codec_id);
    DecodedFrame* amf_DecodeFrame(AmfDecoder* decoder, uint8_t* data, int32_t length);
    void amf_DestroyDecoder(AmfDecoder* decoder);
    int32_t amf_GetWidth(AmfDecoder* decoder);
    int32_t amf_GetHeight(AmfDecoder* decoder);

    void amf_FreeEncodedFrame(EncodedFrame* frame);
    void amf_FreeDecodedFrame(DecodedFrame* frame);
}
