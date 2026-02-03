#pragma once

#include <cstdint>

struct NvEncoder { void* impl; };
struct NvDecoder { void* impl; };
struct EncodedFrame;
struct DecodedFrame;

bool nv_IsEncodeDriverAvailable();
bool nv_IsDecodeDriverAvailable();
/** True when NVDEC decode is available in this build (requires CUDA/NvDecoder integration). */
bool nv_IsDecodeImplemented();

extern "C++" {
    NvEncoder* nv_CreateEncoder(uint8_t* device, int32_t width, int32_t height, int32_t codec_id, int32_t bitrate, int32_t framerate, int32_t gop);
    EncodedFrame* nv_EncodeFrame(NvEncoder* encoder, uint8_t* texture, int64_t timestamp);
    void nv_DestroyEncoder(NvEncoder* encoder);
    void nv_SetBitrate(NvEncoder* encoder, int32_t bitrate);
    void nv_SetFramerate(NvEncoder* encoder, int32_t framerate);

    NvDecoder* nv_CreateDecoder(uint8_t* device, int32_t codec_id);
    DecodedFrame* nv_DecodeFrame(NvDecoder* decoder, uint8_t* data, int32_t length);
    void nv_DestroyDecoder(NvDecoder* decoder);

    void nv_FreeEncodedFrame(EncodedFrame* frame);
    void nv_FreeDecodedFrame(DecodedFrame* frame);
}
