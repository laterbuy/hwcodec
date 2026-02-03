#if !defined(_MSC_VER)
#include <cstdint>
#endif
#include <cstdlib>
#include <cstdio>
#include <cstring>
#include "amf_bridge.h"

#define AMF_DBG(fmt, ...) do { fprintf(stderr, "[AMF] " fmt "\n", ##__VA_ARGS__); fflush(stderr); } while(0)

#if defined(_WIN32) || defined(_WIN64)
#include <windows.h>
#include <d3d11.h>
#endif

#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
#include "core/Factory.h"
#include "core/Context.h"
#include "core/Version.h"
#include "core/Result.h"
#include "core/Platform.h"
#include "core/Buffer.h"
#include "core/Surface.h"
#include "core/Data.h"
#include "core/Variant.h"
#include "core/Plane.h"
#include "components/Component.h"
#include "components/VideoEncoderVCE.h"
#include "components/VideoEncoderHEVC.h"
#include "components/VideoDecoderUVD.h"
using namespace amf;
#endif

struct EncodedFrame {
    uint8_t* data;
    int32_t size;
    bool is_keyframe;
    int64_t timestamp;
};
struct DecodedFrame {
    uint8_t* texture;
    int32_t width;
    int32_t height;
};

static bool IsAmfAvailable() {
#if defined(_WIN32) || defined(_WIN64)
    HMODULE h = LoadLibraryA("amfrt64.dll");
    if (h) {
        FreeLibrary(h);
        return true;
    }
#endif
    return false;
}

extern "C++" bool amf_IsDriverAvailable() {
    return IsAmfAvailable();
}

extern "C++" bool amf_IsDecodeImplemented() {
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    return true;
#else
    return false;
#endif
}

#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
struct AmfEncContext {
    HMODULE dll;
    amf::AMFFactory* factory;
    amf::AMFContext* context;
    amf::AMFComponent* encoder;
    int32_t width;
    int32_t height;
    int32_t codec_id;  // 0 = H.264, 1 = HEVC
};

struct AmfDecContext {
    HMODULE dll;
    amf::AMFFactory* factory;
    amf::AMFContext* context;
    amf::AMFComponent* decoder;
    int32_t width;
    int32_t height;
};
#endif

extern "C++" AmfEncoder* amf_CreateEncoder(uint8_t* device, int32_t width, int32_t height, int32_t codec_id, int32_t bitrate, int32_t framerate, int32_t gop) {
    if (!IsAmfAvailable() || !device || width <= 0 || height <= 0) {
        AMF_DBG("CreateEncoder: 前置条件失败 (available=%d device=%p w=%d h=%d)", IsAmfAvailable() ? 1 : 0, (void*)device, width, height);
        return nullptr;
    }
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    AMF_DBG("CreateEncoder: 使用 AMF 完整实现 (HWCODEC_AMF_FULL)");
    HMODULE dll = LoadLibraryA("amfrt64.dll");
    if (!dll) { AMF_DBG("CreateEncoder: LoadLibrary(amfrt64.dll) 失败"); return nullptr; }
    typedef AMF_RESULT (AMF_CDECL_CALL *AMFInit_Fn)(amf_uint64 version, amf::AMFFactory** ppFactory);
    AMFInit_Fn initFn = (AMFInit_Fn)GetProcAddress(dll, "AMFInit");
    if (!initFn) { AMF_DBG("CreateEncoder: GetProcAddress(AMFInit) 失败"); FreeLibrary(dll); return nullptr; }
    amf::AMFFactory* factory = nullptr;
    AMF_RESULT r = initFn(AMF_FULL_VERSION, &factory);
    if (r != AMF_OK || !factory) { AMF_DBG("CreateEncoder: AMFInit 失败 res=%d", (int)r); FreeLibrary(dll); return nullptr; }
    amf::AMFContext* context = nullptr;
    if (factory->CreateContext(&context) != AMF_OK || !context) { AMF_DBG("CreateEncoder: CreateContext 失败"); FreeLibrary(dll); return nullptr; }
    r = context->InitDX11(device, AMF_DX11_0);
    if (r != AMF_OK) {
        AMF_DBG("CreateEncoder: InitDX11 失败 res=%d (设备可能与 AMF 不兼容)", (int)r);
        context->Release();
        FreeLibrary(dll);
        return nullptr;
    }
    AMFSize size = AMFConstructSize(width, height);
    AMFRate rate = AMFConstructRate((amf_uint32)(framerate > 0 ? framerate : 30), 1);
    amf_int64 bitrateBits = (amf_int64)bitrate * 1000;
    amf_int64 idrPeriod = (gop > 0 && gop < 10000) ? (amf_int64)gop : 60;
    AMF_MEMORY_TYPE memType = AMF_MEMORY_DX11;
    AMF_SURFACE_FORMAT inputFormat = AMF_SURFACE_BGRA;
    amf::AMFComponent* encoder = nullptr;
    if (codec_id == 1) {
        /* HEVC: 部分驱动下对 HEVC 组件调用 SetProperty 会触发 STATUS_ACCESS_VIOLATION，故不设属性直接 Init；
         * Init 因缺少 USAGE 等必填项返回 AMF_FAIL，编码器创建失败但不崩溃。可改用 MFX/NV 的 H.265。 */
        r = factory->CreateComponent(context, AMFVideoEncoder_HEVC, &encoder);
        if (r == AMF_OK && encoder) {
            r = encoder->Init(inputFormat, width, height);
            if (r != AMF_OK) {
                AMF_DBG("CreateEncoder: HEVC Init 失败 res=%d (未设 USAGE 或驱动 SetProperty 会崩溃)", (int)r);
                encoder = nullptr;
            }
        } else {
            AMF_DBG("CreateEncoder: CreateComponent(HEVC) 失败 res=%d", (int)r);
        }
        if (r != AMF_OK || !encoder) {
            context->Release();
            FreeLibrary(dll);
            return nullptr;
        }
    } else {
        r = factory->CreateComponent(context, AMFVideoEncoderVCE_AVC, &encoder);
        if (r != AMF_OK || !encoder) {
            AMF_DBG("CreateEncoder: CreateComponent(AVC) 失败 res=%d", (int)r);
            context->Release();
            FreeLibrary(dll);
            return nullptr;
        }
        AMFVariantStruct varSize, varRate, varBitrate, varGop, varMem;
        AMFVariantInit(&varSize); AMFVariantAssignSize(&varSize, &size);
        encoder->SetProperty(AMF_VIDEO_ENCODER_FRAMESIZE, varSize);
        AMFVariantInit(&varRate); AMFVariantAssignRate(&varRate, &rate);
        encoder->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, varRate);
        AMFVariantInit(&varBitrate); AMFVariantAssignInt64(&varBitrate, bitrateBits);
        encoder->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, varBitrate);
        AMFVariantInit(&varGop); AMFVariantAssignInt64(&varGop, idrPeriod);
        encoder->SetProperty(AMF_VIDEO_ENCODER_IDR_PERIOD, varGop);
        AMFVariantInit(&varMem); AMFVariantAssignInt64(&varMem, (amf_int64)memType);
        encoder->SetProperty(AMF_VIDEO_ENCODER_MEMORY_TYPE, varMem);
        r = encoder->Init(inputFormat, width, height);
        if (r != AMF_OK) {
            AMF_DBG("CreateEncoder: encoder->Init(BGRA %dx%d) 失败 res=%d", width, height, (int)r);
            encoder->Release();
            context->Release();
            FreeLibrary(dll);
            return nullptr;
        }
    }
    AMF_DBG("CreateEncoder: 成功");
    AmfEncContext* ctx = new AmfEncContext();
    ctx->dll = dll;
    ctx->factory = factory;
    ctx->context = context;
    ctx->encoder = encoder;
    ctx->width = width;
    ctx->height = height;
    ctx->codec_id = codec_id;
    AmfEncoder* enc = new AmfEncoder();
    enc->impl = ctx;
    return enc;
#else
    (void)device; (void)width; (void)height; (void)codec_id; (void)bitrate; (void)framerate; (void)gop;
    AMF_DBG("CreateEncoder: 无 externals/AMF_v1.4.35，编码不可用");
    AmfEncoder* enc = new AmfEncoder();
    enc->impl = nullptr;
    return enc;
#endif
}

extern "C++" EncodedFrame* amf_EncodeFrame(AmfEncoder* encoder, uint8_t* texture, int64_t timestamp) {
    if (!encoder || !IsAmfAvailable()) return nullptr;
    if (!encoder->impl) {
        AMF_DBG("EncodeFrame: encoder->impl 为空 (无 AMF SDK 或 CreateEncoder 未成功)");
        return nullptr;
    }
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    if (!texture) { AMF_DBG("EncodeFrame: texture 为空"); return nullptr; }
    AmfEncContext* ctx = (AmfEncContext*)encoder->impl;
    amf::AMFSurface* surface = nullptr;
    AMF_RESULT rSurf = ctx->context->CreateSurfaceFromDX11Native(texture, &surface, nullptr);
    if (rSurf != AMF_OK || !surface) {
        AMF_DBG("EncodeFrame: CreateSurfaceFromDX11Native 失败 res=%d (纹理须为同一 D3D11 设备)", (int)rSurf);
        return nullptr;
    }
    AMFVariantStruct varPts;
    AMFVariantInit(&varPts);
    AMFVariantAssignInt64(&varPts, timestamp);
    surface->SetProperty(AMF_VIDEO_ENCODER_PRESENTATION_TIME_STAMP, varPts);
    AMF_RESULT res = ctx->encoder->SubmitInput(surface);
    if (res == AMF_INPUT_FULL) {
        amf::AMFData* drainData = nullptr;
        for (int i = 0; i < 200; i++) {
            AMF_RESULT qres = ctx->encoder->QueryOutput(&drainData);
            if (qres == AMF_OK && drainData) { drainData->Release(); drainData = nullptr; }
            else if (qres != AMF_NEED_MORE_INPUT) break;
            Sleep(1);
        }
        res = ctx->encoder->SubmitInput(surface);
    }
    surface->Release();
    if (res != AMF_OK) {
        AMF_DBG("EncodeFrame: SubmitInput 失败 res=%d", (int)res);
        return nullptr;
    }
    amf::AMFData* pData = nullptr;
    int queryCount = 0;
    for (int i = 0; i < 500; i++) {
        res = ctx->encoder->QueryOutput(&pData);
        queryCount = i + 1;
        if (res == AMF_OK && pData) break;
        /* AMF_NEED_MORE_INPUT=需更多输入; AMF_REPEAT=请再次调用，继续轮询 */
        if (res != AMF_NEED_MORE_INPUT && res != AMF_REPEAT && res != AMF_OK) {
            AMF_DBG("EncodeFrame: QueryOutput 失败 res=%d (轮询 %d 次)", (int)res, queryCount);
            return nullptr;
        }
        if (pData) { pData->Release(); pData = nullptr; }
        Sleep(1);
    }
    if (res != AMF_OK || !pData) {
        AMF_DBG("EncodeFrame: QueryOutput 超时未取到数据 (res=%d 轮询 %d 次)", (int)res, queryCount);
        return nullptr;
    }
    amf::AMFBuffer* pBuffer = nullptr;
    if (pData->QueryInterface(amf::AMFBuffer::IID(), (void**)&pBuffer) != AMF_OK || !pBuffer) {
        AMF_DBG("EncodeFrame: QueryInterface(AMFBuffer) 失败");
        pData->Release();
        return nullptr;
    }
    amf_size size = pBuffer->GetSize();
    void* ptr = pBuffer->GetNative();
    bool isKeyframe = false;
    AMFVariantStruct varType;
    if (ctx->codec_id == 1) {
        if (pData->GetProperty(AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE, &varType) == AMF_OK) {
            if (varType.type == AMF_VARIANT_INT64 && (varType.int64Value == AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_IDR || varType.int64Value == AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_I)) {
                isKeyframe = true;
            }
        }
    } else {
        if (pData->GetProperty(AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE, &varType) == AMF_OK) {
            if (varType.type == AMF_VARIANT_INT64 && (varType.int64Value == AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_IDR || varType.int64Value == AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_I)) {
                isKeyframe = true;
            }
        }
    }
    EncodedFrame* frame = new EncodedFrame();
    frame->size = (int32_t)size;
    frame->data = (uint8_t*)malloc((size_t)size);
    if (frame->data && size > 0) memcpy(frame->data, ptr, (size_t)size);
    frame->is_keyframe = isKeyframe;
    frame->timestamp = timestamp;
    pBuffer->Release();
    pData->Release();
    return frame;
#else
    (void)texture; (void)timestamp;
    return nullptr;
#endif
}

extern "C++" void amf_FreeEncodedFrame(EncodedFrame* frame) {
    if (!frame) return;
    if (frame->data) { free(frame->data); frame->data = nullptr; }
    delete frame;
}

extern "C++" void amf_FreeDecodedFrame(DecodedFrame* frame) {
    if (frame) delete frame;
}

extern "C++" void amf_DestroyEncoder(AmfEncoder* encoder) {
    if (!encoder) return;
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    if (encoder->impl) {
        AmfEncContext* ctx = (AmfEncContext*)encoder->impl;
        if (ctx->encoder) {
            ctx->encoder->Terminate();
            ctx->encoder->Release();
        }
        if (ctx->context) ctx->context->Release();
        if (ctx->dll) FreeLibrary(ctx->dll);
        delete ctx;
    }
#endif
    encoder->impl = nullptr;
    delete encoder;
}

extern "C++" void amf_SetBitrate(AmfEncoder* encoder, int32_t bitrate) {
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    if (encoder && encoder->impl) {
        AmfEncContext* ctx = (AmfEncContext*)encoder->impl;
        if (ctx->encoder) {
            AMFVariantStruct v;
            AMFVariantInit(&v);
            AMFVariantAssignInt64(&v, (amf_int64)bitrate * 1000);
            if (ctx->codec_id == 1)
                ctx->encoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, v);
            else
                ctx->encoder->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE, v);
        }
    }
#else
    (void)encoder; (void)bitrate;
#endif
}

extern "C++" void amf_SetFramerate(AmfEncoder* encoder, int32_t framerate) {
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    if (encoder && encoder->impl) {
        AmfEncContext* ctx = (AmfEncContext*)encoder->impl;
        if (ctx->encoder) {
            AMFRate rate = AMFConstructRate((amf_uint32)(framerate > 0 ? framerate : 30), 1);
            AMFVariantStruct v;
            AMFVariantInit(&v);
            AMFVariantAssignRate(&v, &rate);
            if (ctx->codec_id == 1)
                ctx->encoder->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, v);
            else
                ctx->encoder->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, v);
        }
    }
#else
    (void)encoder; (void)framerate;
#endif
}

// AmfDecoder: full implementation when HWCODEC_AMF_FULL
extern "C++" AmfDecoder* amf_CreateDecoder(uint8_t* device, int32_t codec_id) {
    if (!IsAmfAvailable() || !device) return nullptr;
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    HMODULE dll = LoadLibraryA("amfrt64.dll");
    if (!dll) return nullptr;
    typedef AMF_RESULT (AMF_CDECL_CALL *AMFInit_Fn)(amf_uint64 version, amf::AMFFactory** ppFactory);
    AMFInit_Fn initFn = (AMFInit_Fn)GetProcAddress(dll, "AMFInit");
    if (!initFn) { FreeLibrary(dll); return nullptr; }
    amf::AMFFactory* factory = nullptr;
    if (initFn(AMF_FULL_VERSION, &factory) != AMF_OK || !factory) { FreeLibrary(dll); return nullptr; }
    amf::AMFContext* context = nullptr;
    if (factory->CreateContext(&context) != AMF_OK || !context) { FreeLibrary(dll); return nullptr; }
    if (context->InitDX11(device, AMF_DX11_0) != AMF_OK) {
        context->Release(); FreeLibrary(dll); return nullptr;
    }
    const wchar_t* decoderId = (codec_id == 1) ? AMFVideoDecoderHW_H265_HEVC : AMFVideoDecoderUVD_H264_AVC;
    amf::AMFComponent* decoder = nullptr;
    if (factory->CreateComponent(context, decoderId, &decoder) != AMF_OK || !decoder) {
        context->Release(); FreeLibrary(dll); return nullptr;
    }
    AMF_RESULT r = decoder->Init(AMF_SURFACE_NV12, 0, 0);
    if (r != AMF_OK) {
        AMF_DBG("CreateDecoder: decoder->Init failed res=%d", (int)r);
        decoder->Release(); context->Release(); FreeLibrary(dll); return nullptr;
    }
    AmfDecContext* ctx = new AmfDecContext();
    ctx->dll = dll;
    ctx->factory = factory;
    ctx->context = context;
    ctx->decoder = decoder;
    ctx->width = 0;
    ctx->height = 0;
    AmfDecoder* dec = new AmfDecoder();
    dec->impl = ctx;
    AMF_DBG("CreateDecoder: ok");
    return dec;
#else
    AmfDecoder* dec = new AmfDecoder();
    dec->impl = nullptr;
    return dec;
#endif
}

extern "C++" DecodedFrame* amf_DecodeFrame(AmfDecoder* decoder, uint8_t* data, int32_t length) {
    if (!decoder || !IsAmfAvailable() || !data || length <= 0) return nullptr;
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    if (!decoder->impl) return nullptr;
    AmfDecContext* ctx = (AmfDecContext*)decoder->impl;
    amf::AMFBuffer* pBuffer = nullptr;
    AMF_RESULT r = ctx->context->AllocBuffer(AMF_MEMORY_HOST, (amf_size)length, &pBuffer);
    if (r != AMF_OK || !pBuffer) return nullptr;
    void* dst = pBuffer->GetNative();
    if (dst && length > 0) memcpy(dst, data, (size_t)length);
    r = ctx->decoder->SubmitInput(pBuffer);
    pBuffer->Release();
    if (r == AMF_INPUT_FULL) {
        amf::AMFData* drain = nullptr;
        for (int i = 0; i < 64; i++) {
            if (ctx->decoder->QueryOutput(&drain) == AMF_OK && drain) { drain->Release(); drain = nullptr; }
            Sleep(1);
        }
        r = ctx->context->AllocBuffer(AMF_MEMORY_HOST, (amf_size)length, &pBuffer);
        if (r != AMF_OK || !pBuffer) return nullptr;
        void* dst = pBuffer->GetNative();
        if (dst && length > 0) memcpy(dst, data, (size_t)length);
        r = ctx->decoder->SubmitInput(pBuffer);
        pBuffer->Release();
    }
    if (r != AMF_OK) return nullptr;
    amf::AMFData* pData = nullptr;
    for (int i = 0; i < 200; i++) {
        r = ctx->decoder->QueryOutput(&pData);
        if (r == AMF_OK && pData) break;
        if (r != AMF_NEED_MORE_INPUT && r != AMF_REPEAT) return nullptr;
        if (pData) { pData->Release(); pData = nullptr; }
        Sleep(1);
    }
    if (r != AMF_OK || !pData) return nullptr;
    amf::AMFSurface* pSurface = nullptr;
    if (pData->QueryInterface(amf::AMFSurface::IID(), (void**)&pSurface) != AMF_OK || !pSurface) {
        pData->Release(); return nullptr;
    }
    AMFPlane* plane = pSurface->GetPlaneAt(0);
    if (!plane) { pSurface->Release(); pData->Release(); return nullptr; }
    void* native = plane->GetNative();
    int32_t w = (int32_t)plane->GetWidth();
    int32_t h = (int32_t)plane->GetHeight();
    if (ctx->width == 0 || ctx->height == 0) { ctx->width = w; ctx->height = h; }
    pSurface->Release();
    pData->Release();
    if (!native) return nullptr;
    DecodedFrame* frame = new DecodedFrame();
    frame->texture = (uint8_t*)native;
    frame->width = w;
    frame->height = h;
    return frame;
#else
    (void)data; (void)length;
    return nullptr;
#endif
}

extern "C++" void amf_DestroyDecoder(AmfDecoder* decoder) {
    if (!decoder) return;
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    if (decoder->impl) {
        AmfDecContext* ctx = (AmfDecContext*)decoder->impl;
        if (ctx->decoder) { ctx->decoder->Terminate(); ctx->decoder->Release(); }
        if (ctx->context) ctx->context->Release();
        if (ctx->dll) FreeLibrary(ctx->dll);
        delete ctx;
    }
#endif
    decoder->impl = nullptr;
    delete decoder;
}

extern "C++" int32_t amf_GetWidth(AmfDecoder* decoder) {
    if (!decoder || !decoder->impl) return 0;
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    return ((AmfDecContext*)decoder->impl)->width;
#else
    return 0;
#endif
}

extern "C++" int32_t amf_GetHeight(AmfDecoder* decoder) {
    if (!decoder || !decoder->impl) return 0;
#if defined(_WIN32) && defined(_WIN64) && defined(HWCODEC_AMF_FULL)
    return ((AmfDecContext*)decoder->impl)->height;
#else
    return 0;
#endif
}
