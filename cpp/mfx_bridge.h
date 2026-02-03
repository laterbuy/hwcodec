#pragma once

#include <cstdint>

struct MfxEncoder { void* impl; };
struct MfxDecoder { void* impl; };
struct EncodedFrame;
struct DecodedFrame;

bool mfx_IsDriverAvailable();

extern "C++" {
    MfxEncoder* mfx_CreateEncoder(uint8_t* device, int32_t width, int32_t height, int32_t codec_id, int32_t bitrate, int32_t framerate, int32_t gop);
    EncodedFrame* mfx_EncodeFrame(MfxEncoder* encoder, uint8_t* texture, int64_t timestamp);
    void mfx_DestroyEncoder(MfxEncoder* encoder);
    void mfx_SetBitrate(MfxEncoder* encoder, int32_t bitrate);
    void mfx_SetFramerate(MfxEncoder* encoder, int32_t framerate);

    MfxDecoder* mfx_CreateDecoder(uint8_t* device, int32_t codec_id);
    DecodedFrame* mfx_DecodeFrame(MfxDecoder* decoder, uint8_t* data, int32_t length);
    void mfx_DestroyDecoder(MfxDecoder* decoder);
    int32_t mfx_GetWidth(MfxDecoder* decoder);
    int32_t mfx_GetHeight(MfxDecoder* decoder);

    void mfx_FreeEncodedFrame(EncodedFrame* frame);
    void mfx_FreeDecodedFrame(DecodedFrame* frame);
}
