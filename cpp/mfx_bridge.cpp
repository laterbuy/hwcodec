#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include "mfx_bridge.h"

#if defined(_WIN32) || defined(_WIN64)
#include <windows.h>
#include <d3d11.h>
#endif

#define MFX_DBG(fmt, ...) do { fprintf(stderr, "[MFX] " fmt "\n", ##__VA_ARGS__); fflush(stderr); } while(0)

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

static bool IsMfxAvailable() {
#if defined(_WIN32) || defined(_WIN64)
    HMODULE h = LoadLibraryA("mfx.dll");
    if (h) {
        FreeLibrary(h);
        return true;
    }
    h = LoadLibraryA("libmfxhw64.dll");
    if (h) {
        FreeLibrary(h);
        return true;
    }
#endif
    return false;
}

extern "C++" bool mfx_IsDriverAvailable() {
    return IsMfxAvailable();
}

#if defined(_WIN32) || defined(_WIN64)
#include "mfxvideo.h"
#include "mfxstructures.h"
#include "mfxsession.h"

static HMODULE s_mfx_dll = nullptr;

typedef mfxStatus (MFX_CDECL *Fn_MFXInitEx)(mfxInitParam par, mfxSession *session);
typedef mfxStatus (MFX_CDECL *Fn_MFXClose)(mfxSession session);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoCORE_SetHandle)(mfxSession session, mfxHandleType type, mfxHDL hdl);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoCORE_SyncOperation)(mfxSession session, mfxSyncPoint syncp, mfxU32 wait);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoENCODE_Query)(mfxSession session, mfxVideoParam *in, mfxVideoParam *out);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoENCODE_QueryIOSurf)(mfxSession session, mfxVideoParam *par, mfxFrameAllocRequest *request);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoENCODE_Init)(mfxSession session, mfxVideoParam *par);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoENCODE_Reset)(mfxSession session, mfxVideoParam *par);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoENCODE_Close)(mfxSession session);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoENCODE_EncodeFrameAsync)(mfxSession session, mfxEncodeCtrl *ctrl, mfxFrameSurface1 *surface, mfxBitstream *bs, mfxSyncPoint *syncp);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoCORE_SetFrameAllocator)(mfxSession session, mfxFrameAllocator *allocator);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoDECODE_Query)(mfxSession session, mfxVideoParam *in, mfxVideoParam *out);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoDECODE_DecodeHeader)(mfxSession session, mfxBitstream *bs, mfxVideoParam *par);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoDECODE_QueryIOSurf)(mfxSession session, mfxVideoParam *par, mfxFrameAllocRequest *request);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoDECODE_Init)(mfxSession session, mfxVideoParam *par);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoDECODE_DecodeFrameAsync)(mfxSession session, mfxBitstream *bs, mfxFrameSurface1 *surface_work, mfxFrameSurface1 **surface_out, mfxSyncPoint *syncp);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoDECODE_Close)(mfxSession session);
typedef mfxStatus (MFX_CDECL *Fn_MFXVideoDECODE_GetVideoParam)(mfxSession session, mfxVideoParam *par);

static Fn_MFXInitEx pMFXInitEx = nullptr;
static Fn_MFXClose pMFXClose = nullptr;
static Fn_MFXVideoCORE_SetHandle pMFXVideoCORE_SetHandle = nullptr;
static Fn_MFXVideoCORE_SyncOperation pMFXVideoCORE_SyncOperation = nullptr;
static Fn_MFXVideoENCODE_Query pMFXVideoENCODE_Query = nullptr;
static Fn_MFXVideoENCODE_QueryIOSurf pMFXVideoENCODE_QueryIOSurf = nullptr;
static Fn_MFXVideoENCODE_Init pMFXVideoENCODE_Init = nullptr;
static Fn_MFXVideoENCODE_Reset pMFXVideoENCODE_Reset = nullptr;
static Fn_MFXVideoENCODE_Close pMFXVideoENCODE_Close = nullptr;
static Fn_MFXVideoENCODE_EncodeFrameAsync pMFXVideoENCODE_EncodeFrameAsync = nullptr;
static Fn_MFXVideoCORE_SetFrameAllocator pMFXVideoCORE_SetFrameAllocator = nullptr;
static Fn_MFXVideoDECODE_Query pMFXVideoDECODE_Query = nullptr;
static Fn_MFXVideoDECODE_DecodeHeader pMFXVideoDECODE_DecodeHeader = nullptr;
static Fn_MFXVideoDECODE_QueryIOSurf pMFXVideoDECODE_QueryIOSurf = nullptr;
static Fn_MFXVideoDECODE_Init pMFXVideoDECODE_Init = nullptr;
static Fn_MFXVideoDECODE_DecodeFrameAsync pMFXVideoDECODE_DecodeFrameAsync = nullptr;
static Fn_MFXVideoDECODE_Close pMFXVideoDECODE_Close = nullptr;
static Fn_MFXVideoDECODE_GetVideoParam pMFXVideoDECODE_GetVideoParam = nullptr;

static bool LoadMfxProcs() {
    if (pMFXInitEx) return true;
    const char* dlls[] = { "mfx.dll", "libmfxhw64.dll" };
    for (int i = 0; i < 2; i++) {
        s_mfx_dll = LoadLibraryA(dlls[i]);
        if (!s_mfx_dll) continue;
        pMFXInitEx = (Fn_MFXInitEx)GetProcAddress(s_mfx_dll, "MFXInitEx");
        pMFXClose = (Fn_MFXClose)GetProcAddress(s_mfx_dll, "MFXClose");
        pMFXVideoCORE_SetHandle = (Fn_MFXVideoCORE_SetHandle)GetProcAddress(s_mfx_dll, "MFXVideoCORE_SetHandle");
        pMFXVideoCORE_SyncOperation = (Fn_MFXVideoCORE_SyncOperation)GetProcAddress(s_mfx_dll, "MFXVideoCORE_SyncOperation");
        pMFXVideoENCODE_Query = (Fn_MFXVideoENCODE_Query)GetProcAddress(s_mfx_dll, "MFXVideoENCODE_Query");
        pMFXVideoENCODE_QueryIOSurf = (Fn_MFXVideoENCODE_QueryIOSurf)GetProcAddress(s_mfx_dll, "MFXVideoENCODE_QueryIOSurf");
        pMFXVideoENCODE_Init = (Fn_MFXVideoENCODE_Init)GetProcAddress(s_mfx_dll, "MFXVideoENCODE_Init");
        pMFXVideoENCODE_Reset = (Fn_MFXVideoENCODE_Reset)GetProcAddress(s_mfx_dll, "MFXVideoENCODE_Reset");
        pMFXVideoENCODE_Close = (Fn_MFXVideoENCODE_Close)GetProcAddress(s_mfx_dll, "MFXVideoENCODE_Close");
        pMFXVideoENCODE_EncodeFrameAsync = (Fn_MFXVideoENCODE_EncodeFrameAsync)GetProcAddress(s_mfx_dll, "MFXVideoENCODE_EncodeFrameAsync");
        pMFXVideoCORE_SetFrameAllocator = (Fn_MFXVideoCORE_SetFrameAllocator)GetProcAddress(s_mfx_dll, "MFXVideoCORE_SetFrameAllocator");
        pMFXVideoDECODE_Query = (Fn_MFXVideoDECODE_Query)GetProcAddress(s_mfx_dll, "MFXVideoDECODE_Query");
        pMFXVideoDECODE_DecodeHeader = (Fn_MFXVideoDECODE_DecodeHeader)GetProcAddress(s_mfx_dll, "MFXVideoDECODE_DecodeHeader");
        pMFXVideoDECODE_QueryIOSurf = (Fn_MFXVideoDECODE_QueryIOSurf)GetProcAddress(s_mfx_dll, "MFXVideoDECODE_QueryIOSurf");
        pMFXVideoDECODE_Init = (Fn_MFXVideoDECODE_Init)GetProcAddress(s_mfx_dll, "MFXVideoDECODE_Init");
        pMFXVideoDECODE_DecodeFrameAsync = (Fn_MFXVideoDECODE_DecodeFrameAsync)GetProcAddress(s_mfx_dll, "MFXVideoDECODE_DecodeFrameAsync");
        pMFXVideoDECODE_Close = (Fn_MFXVideoDECODE_Close)GetProcAddress(s_mfx_dll, "MFXVideoDECODE_Close");
        pMFXVideoDECODE_GetVideoParam = (Fn_MFXVideoDECODE_GetVideoParam)GetProcAddress(s_mfx_dll, "MFXVideoDECODE_GetVideoParam");
        if (pMFXInitEx && pMFXClose && pMFXVideoENCODE_Init && pMFXVideoENCODE_EncodeFrameAsync && pMFXVideoCORE_SyncOperation) {
            MFX_DBG("Loaded %s", dlls[i]);
            return true;
        }
        FreeLibrary(s_mfx_dll);
        s_mfx_dll = nullptr;
    }
    return false;
}

/* Minimal D3D11 pass-through allocator: input surfaces use MemId = (mfxMemId)ID3D11Texture2D*; GetHDL returns it. */
static mfxStatus MFX_CDECL AllocPassthrough(mfxHDL pthis, mfxFrameAllocRequest *request, mfxFrameAllocResponse *response) {
    (void)pthis;
    (void)request;
    (void)response;
    return MFX_ERR_UNSUPPORTED;
}
static mfxStatus MFX_CDECL LockPassthrough(mfxHDL pthis, mfxMemId mid, mfxFrameData *ptr) {
    (void)pthis;
    (void)mid;
    (void)ptr;
    return MFX_ERR_UNSUPPORTED;
}
static mfxStatus MFX_CDECL UnlockPassthrough(mfxHDL pthis, mfxMemId mid, mfxFrameData *ptr) {
    (void)pthis;
    (void)mid;
    (void)ptr;
    return MFX_ERR_NONE;
}
static mfxStatus MFX_CDECL GetHDLPassthrough(mfxHDL pthis, mfxMemId mid, mfxHDL *handle) {
    (void)pthis;
    *handle = mid;
    return MFX_ERR_NONE;
}
static mfxStatus MFX_CDECL FreePassthrough(mfxHDL pthis, mfxFrameAllocResponse *response) {
    (void)pthis;
    (void)response;
    return MFX_ERR_NONE;
}

static mfxFrameAllocator s_passthrough_allocator = {
    {0}, nullptr,
    AllocPassthrough, LockPassthrough, UnlockPassthrough, GetHDLPassthrough, FreePassthrough
};

/* Decode allocator: allocates D3D11 NV12 textures for decoder output */
struct DecAllocContext {
    ID3D11Device* dev;
    ID3D11Texture2D** textures;
    mfxMemId* mids;
    mfxU16 num;
};

static mfxStatus MFX_CDECL DecAlloc_Alloc(mfxHDL pthis, mfxFrameAllocRequest* request, mfxFrameAllocResponse* response) {
    DecAllocContext* ctx = (DecAllocContext*)pthis;
    if (!ctx || !ctx->dev || !request || !response) return MFX_ERR_NULL_PTR;
    mfxU16 n = request->NumFrameSuggested;
    if (n == 0) n = 4;
    mfxU32 w = request->Info.Width;
    mfxU32 h = request->Info.Height;
    if (w == 0 || h == 0) return MFX_ERR_UNSUPPORTED;
    ID3D11Texture2D** texs = (ID3D11Texture2D**)malloc(sizeof(ID3D11Texture2D*) * (size_t)n);
    if (!texs) return MFX_ERR_MEMORY_ALLOC;
    memset(texs, 0, sizeof(ID3D11Texture2D*) * (size_t)n);
    D3D11_TEXTURE2D_DESC desc = {};
    desc.Width = w;
    desc.Height = h;
    desc.MipLevels = 1;
    desc.ArraySize = 1;
    desc.Format = (DXGI_FORMAT)0x3231564E; /* DXGI_FORMAT_NV12 */
    desc.SampleDesc.Count = 1;
    desc.SampleDesc.Quality = 0;
    desc.Usage = D3D11_USAGE_DEFAULT;
    desc.BindFlags = D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;
    desc.CPUAccessFlags = 0;
    desc.MiscFlags = 0;
    for (mfxU16 i = 0; i < n; i++) {
        if (FAILED(ctx->dev->CreateTexture2D(&desc, nullptr, &texs[i]))) {
            for (mfxU16 j = 0; j < i; j++) texs[j]->Release();
            free(texs);
            return MFX_ERR_MEMORY_ALLOC;
        }
    }
    mfxMemId* mids = (mfxMemId*)malloc(sizeof(mfxMemId) * (size_t)n);
    if (!mids) {
        for (mfxU16 i = 0; i < n; i++) texs[i]->Release();
        free(texs);
        return MFX_ERR_MEMORY_ALLOC;
    }
    for (mfxU16 i = 0; i < n; i++) mids[i] = (mfxMemId)texs[i];
    ctx->textures = texs;
    ctx->mids = mids;
    ctx->num = n;
    response->mids = mids;
    response->NumFrameActual = n;
    return MFX_ERR_NONE;
}
static mfxStatus MFX_CDECL DecAlloc_Lock(mfxHDL pthis, mfxMemId mid, mfxFrameData* ptr) {
    (void)pthis; (void)mid; (void)ptr;
    return MFX_ERR_UNSUPPORTED;
}
static mfxStatus MFX_CDECL DecAlloc_Unlock(mfxHDL pthis, mfxMemId mid, mfxFrameData* ptr) {
    (void)pthis; (void)mid; (void)ptr;
    return MFX_ERR_NONE;
}
static mfxStatus MFX_CDECL DecAlloc_GetHDL(mfxHDL pthis, mfxMemId mid, mfxHDL* handle) {
    (void)pthis;
    *handle = mid;
    return MFX_ERR_NONE;
}
static mfxStatus MFX_CDECL DecAlloc_Free(mfxHDL pthis, mfxFrameAllocResponse* response) {
    DecAllocContext* ctx = (DecAllocContext*)pthis;
    if (!ctx) return MFX_ERR_NONE;
    if (ctx->textures && response && response->mids) {
        for (mfxU16 i = 0; i < ctx->num; i++)
            if (ctx->textures[i]) ctx->textures[i]->Release();
        free(ctx->textures);
        ctx->textures = nullptr;
    }
    if (ctx->mids) { free(ctx->mids); ctx->mids = nullptr; }
    ctx->num = 0;
    return MFX_ERR_NONE;
}

static mfxFrameAllocator s_decode_allocator = {
    {0}, nullptr,
    DecAlloc_Alloc, DecAlloc_Lock, DecAlloc_Unlock, DecAlloc_GetHDL, DecAlloc_Free
};

/* Encoder context */
struct MfxEncContext {
    mfxSession session;
    mfxVideoParam param;
    int32_t width;
    int32_t height;
    uint8_t* bs_buffer;
    mfxU32 bs_buffer_size;
};

/* Decoder context */
struct MfxDecContext {
    mfxSession session;
    mfxVideoParam param;
    int32_t width;
    int32_t height;
    DecAllocContext alloc_ctx;
    mfxFrameAllocator allocator;
};
#endif

extern "C++" MfxEncoder* mfx_CreateEncoder(uint8_t* device, int32_t width, int32_t height, int32_t codec_id, int32_t bitrate, int32_t framerate, int32_t gop) {
    if (!IsMfxAvailable() || !device || width <= 0 || height <= 0) return nullptr;
#if defined(_WIN32) || defined(_WIN64)
    if (!LoadMfxProcs()) {
        MFX_DBG("CreateEncoder: LoadMfxProcs failed");
        return nullptr;
    }
    mfxInitParam initPar = {};
    initPar.Implementation = MFX_IMPL_HARDWARE | MFX_IMPL_VIA_D3D11;
    initPar.Version.Major = 1;
    initPar.Version.Minor = 35;
    mfxSession session = nullptr;
    mfxStatus st = pMFXInitEx(initPar, &session);
    if (st != MFX_ERR_NONE || !session) {
        MFX_DBG("CreateEncoder: MFXInitEx failed st=%d", (int)st);
        return nullptr;
    }
    st = pMFXVideoCORE_SetHandle(session, MFX_HANDLE_D3D11_DEVICE, (mfxHDL)device);
    if (st != MFX_ERR_NONE) {
        MFX_DBG("CreateEncoder: SetHandle(D3D11) failed st=%d", (int)st);
        pMFXClose(session);
        return nullptr;
    }
    if (pMFXVideoCORE_SetFrameAllocator) {
        s_passthrough_allocator.pthis = nullptr;
        pMFXVideoCORE_SetFrameAllocator(session, &s_passthrough_allocator);
    }
    mfxVideoParam param = {};
    param.mfx.CodecId = (codec_id == 0) ? MFX_CODEC_AVC : MFX_CODEC_HEVC;
    if (codec_id == 0) {
        param.mfx.CodecProfile = MFX_PROFILE_AVC_HIGH;
        param.mfx.CodecLevel = MFX_LEVEL_AVC_41;
    } else {
        param.mfx.CodecProfile = MFX_PROFILE_HEVC_MAIN;
        param.mfx.CodecLevel = MFX_LEVEL_HEVC_41;
    }
    param.mfx.FrameInfo.FourCC = MFX_FOURCC_NV12;
    param.mfx.FrameInfo.Width = (mfxU16)width;
    param.mfx.FrameInfo.Height = (mfxU16)height;
    param.mfx.FrameInfo.CropW = (mfxU16)width;
    param.mfx.FrameInfo.CropH = (mfxU16)height;
    param.mfx.FrameInfo.FrameRateExtN = (mfxU32)(framerate > 0 ? framerate : 30);
    param.mfx.FrameInfo.FrameRateExtD = 1;
    param.mfx.FrameInfo.PicStruct = MFX_PICSTRUCT_PROGRESSIVE;
    param.mfx.FrameInfo.ChromaFormat = MFX_CHROMAFORMAT_YUV420;
    param.mfx.GopPicSize = (mfxU16)(gop > 0 && gop < 10000 ? gop : 60);
    param.mfx.GopRefDist = 1;
    param.mfx.RateControlMethod = MFX_RATECONTROL_CBR;
    param.mfx.TargetKbps = (mfxU16)(bitrate > 0 ? (bitrate / 1000) : 4000);
    param.IOPattern = MFX_IOPATTERN_IN_VIDEO_MEMORY;
    param.AsyncDepth = 1;
    mfxVideoParam outParam = {};
    st = pMFXVideoENCODE_Query(session, &param, &outParam);
    if (st != MFX_ERR_NONE) {
        MFX_DBG("CreateEncoder: ENCODE_Query failed st=%d", (int)st);
        pMFXClose(session);
        return nullptr;
    }
    st = pMFXVideoENCODE_Init(session, &param);
    if (st != MFX_ERR_NONE) {
        MFX_DBG("CreateEncoder: ENCODE_Init failed st=%d", (int)st);
        pMFXClose(session);
        return nullptr;
    }
    MfxEncContext* ctx = new MfxEncContext();
    ctx->session = session;
    ctx->param = param;
    ctx->width = width;
    ctx->height = height;
    ctx->bs_buffer_size = (mfxU32)(width * height * 2);
    if (ctx->bs_buffer_size < 200000) ctx->bs_buffer_size = 200000;
    ctx->bs_buffer = (uint8_t*)malloc(ctx->bs_buffer_size);
    MfxEncoder* enc = new MfxEncoder();
    enc->impl = ctx;
    MFX_DBG("CreateEncoder: ok %dx%d", width, height);
    return enc;
#else
    (void)device; (void)width; (void)height; (void)codec_id; (void)bitrate; (void)framerate; (void)gop;
    MfxEncoder* enc = new MfxEncoder();
    enc->impl = nullptr;
    return enc;
#endif
}

extern "C++" EncodedFrame* mfx_EncodeFrame(MfxEncoder* encoder, uint8_t* texture, int64_t timestamp) {
    if (!encoder || !IsMfxAvailable()) return nullptr;
    if (!encoder->impl) return nullptr;
#if defined(_WIN32) || defined(_WIN64)
    if (!texture || !LoadMfxProcs()) return nullptr;
    MfxEncContext* ctx = (MfxEncContext*)encoder->impl;
    mfxFrameSurface1 surf = {};
    surf.Info.FourCC = MFX_FOURCC_NV12;
    surf.Info.Width = (mfxU16)ctx->width;
    surf.Info.Height = (mfxU16)ctx->height;
    surf.Info.CropW = (mfxU16)ctx->width;
    surf.Info.CropH = (mfxU16)ctx->height;
    surf.Info.FrameRateExtN = ctx->param.mfx.FrameInfo.FrameRateExtN;
    surf.Info.FrameRateExtD = ctx->param.mfx.FrameInfo.FrameRateExtD;
    surf.Info.PicStruct = MFX_PICSTRUCT_PROGRESSIVE;
    surf.Info.ChromaFormat = MFX_CHROMAFORMAT_YUV420;
    surf.Data.MemId = (mfxMemId)texture;
    surf.Data.TimeStamp = (mfxU64)timestamp;
    mfxBitstream bs = {};
    bs.Data = ctx->bs_buffer;
    bs.MaxLength = ctx->bs_buffer_size;
    bs.DataOffset = 0;
    bs.DataLength = 0;
    mfxSyncPoint syncp = nullptr;
    mfxStatus st = pMFXVideoENCODE_EncodeFrameAsync(ctx->session, nullptr, &surf, &bs, &syncp);
    if (st == MFX_ERR_MORE_DATA) return nullptr;
    if (st == MFX_ERR_MORE_BITSTREAM) {
        MFX_DBG("EncodeFrame: output buffer too small");
        return nullptr;
    }
    if (st != MFX_ERR_NONE) {
        MFX_DBG("EncodeFrame: EncodeFrameAsync st=%d", (int)st);
        return nullptr;
    }
    st = pMFXVideoCORE_SyncOperation(ctx->session, syncp, 3000);
    if (st != MFX_ERR_NONE) return nullptr;
    EncodedFrame* frame = new EncodedFrame();
    frame->size = (int32_t)bs.DataLength;
    frame->data = (uint8_t*)malloc((size_t)bs.DataLength);
    if (frame->data && bs.DataLength > 0)
        memcpy(frame->data, bs.Data + bs.DataOffset, (size_t)bs.DataLength);
    frame->is_keyframe = (bs.FrameType & MFX_FRAMETYPE_IDR) != 0;
    frame->timestamp = timestamp;
    return frame;
#else
    (void)encoder; (void)texture; (void)timestamp;
    return nullptr;
#endif
}

extern "C++" void mfx_FreeEncodedFrame(EncodedFrame* frame) {
    if (!frame) return;
    if (frame->data) { free(frame->data); frame->data = nullptr; }
    delete frame;
}

extern "C++" void mfx_FreeDecodedFrame(DecodedFrame* frame) {
    if (frame) delete frame;
}

extern "C++" void mfx_DestroyEncoder(MfxEncoder* encoder) {
    if (!encoder) return;
#if defined(_WIN32) || defined(_WIN64)
    if (encoder->impl) {
        MfxEncContext* ctx = (MfxEncContext*)encoder->impl;
        if (pMFXVideoENCODE_Close) pMFXVideoENCODE_Close(ctx->session);
        if (pMFXClose) pMFXClose(ctx->session);
        if (ctx->bs_buffer) free(ctx->bs_buffer);
        delete ctx;
    }
#endif
    encoder->impl = nullptr;
    delete encoder;
}

extern "C++" void mfx_SetBitrate(MfxEncoder* encoder, int32_t bitrate) {
    if (!encoder || !encoder->impl || !IsMfxAvailable()) return;
#if defined(_WIN32) || defined(_WIN64)
    if (!LoadMfxProcs() || !pMFXVideoENCODE_Reset) return;
    MfxEncContext* ctx = (MfxEncContext*)encoder->impl;
    ctx->param.mfx.TargetKbps = (mfxU16)(bitrate > 0 ? (bitrate / 1000) : 4000);
    pMFXVideoENCODE_Reset(ctx->session, &ctx->param);
#endif
}

extern "C++" void mfx_SetFramerate(MfxEncoder* encoder, int32_t framerate) {
    if (!encoder || !encoder->impl || !IsMfxAvailable()) return;
#if defined(_WIN32) || defined(_WIN64)
    if (!LoadMfxProcs() || !pMFXVideoENCODE_Reset) return;
    MfxEncContext* ctx = (MfxEncContext*)encoder->impl;
    ctx->param.mfx.FrameInfo.FrameRateExtN = (mfxU32)(framerate > 0 ? framerate : 30);
    ctx->param.mfx.FrameInfo.FrameRateExtD = 1;
    pMFXVideoENCODE_Reset(ctx->session, &ctx->param);
#endif
}

/* Decoder: init with first chunk to get width/height; decode returns output surface's texture. */
extern "C++" MfxDecoder* mfx_CreateDecoder(uint8_t* device, int32_t codec_id) {
    if (!IsMfxAvailable() || !device) return nullptr;
#if defined(_WIN32) || defined(_WIN64)
    if (!LoadMfxProcs()) return nullptr;
    mfxInitParam initPar = {};
    initPar.Implementation = MFX_IMPL_HARDWARE | MFX_IMPL_VIA_D3D11;
    initPar.Version.Major = 1;
    initPar.Version.Minor = 35;
    mfxSession session = nullptr;
    mfxStatus st = pMFXInitEx(initPar, &session);
    if (st != MFX_ERR_NONE || !session) return nullptr;
    st = pMFXVideoCORE_SetHandle(session, MFX_HANDLE_D3D11_DEVICE, (mfxHDL)device);
    if (st != MFX_ERR_NONE) {
        pMFXClose(session);
        return nullptr;
    }
    MfxDecContext* ctx = new MfxDecContext();
    ctx->session = session;
    ctx->param = {};
    ctx->param.mfx.CodecId = (codec_id == 0) ? MFX_CODEC_AVC : MFX_CODEC_HEVC;
    ctx->width = 0;
    ctx->height = 0;
    ctx->alloc_ctx.dev = (ID3D11Device*)device;
    ctx->alloc_ctx.textures = nullptr;
    ctx->alloc_ctx.mids = nullptr;
    ctx->alloc_ctx.num = 0;
    ctx->allocator = s_decode_allocator;
    ctx->allocator.pthis = &ctx->alloc_ctx;
    if (pMFXVideoCORE_SetFrameAllocator)
        pMFXVideoCORE_SetFrameAllocator(session, &ctx->allocator);
    MfxDecoder* dec = new MfxDecoder();
    dec->impl = ctx;
    MFX_DBG("CreateDecoder: session ok, call DecodeFrame with first NAL to init");
    return dec;
#else
    (void)device; (void)codec_id;
    MfxDecoder* dec = new MfxDecoder();
    dec->impl = nullptr;
    return dec;
#endif
}

extern "C++" DecodedFrame* mfx_DecodeFrame(MfxDecoder* decoder, uint8_t* data, int32_t length) {
    if (!decoder || !decoder->impl || !IsMfxAvailable() || !data || length <= 0) return nullptr;
#if defined(_WIN32) || defined(_WIN64)
    if (!LoadMfxProcs()) return nullptr;
    MfxDecContext* ctx = (MfxDecContext*)decoder->impl;
    if (ctx->width == 0 && pMFXVideoDECODE_DecodeHeader) {
        mfxBitstream bs = {};
        bs.Data = data;
        bs.DataLength = (mfxU32)length;
        bs.MaxLength = (mfxU32)length;
        bs.DataOffset = 0;
        mfxVideoParam par = {};
        mfxStatus st = pMFXVideoDECODE_DecodeHeader(ctx->session, &bs, &par);
        if (st != MFX_ERR_NONE) return nullptr;
        ctx->param = par;
        ctx->param.IOPattern = MFX_IOPATTERN_OUT_VIDEO_MEMORY;
        ctx->param.AsyncDepth = 1;
        st = pMFXVideoDECODE_Init(ctx->session, &ctx->param);
        if (st != MFX_ERR_NONE) {
            MFX_DBG("DecodeFrame: DECODE_Init failed st=%d", (int)st);
            return nullptr;
        }
        ctx->width = (int32_t)par.mfx.FrameInfo.CropW;
        ctx->height = (int32_t)par.mfx.FrameInfo.CropH;
      }
    mfxBitstream bs = {};
    bs.Data = data;
    bs.DataLength = (mfxU32)length;
    bs.MaxLength = (mfxU32)length;
    bs.DataOffset = 0;
    mfxFrameSurface1* surface_out = nullptr;
    mfxSyncPoint syncp = nullptr;
    mfxStatus st = pMFXVideoDECODE_DecodeFrameAsync(ctx->session, &bs, nullptr, &surface_out, &syncp);
    if (st == MFX_ERR_MORE_DATA) return nullptr;
    if (st != MFX_ERR_NONE || !surface_out) return nullptr;
    st = pMFXVideoCORE_SyncOperation(ctx->session, syncp, 3000);
    if (st != MFX_ERR_NONE) return nullptr;
    mfxHDL hdl = nullptr;
    ctx->allocator.GetHDL(ctx->allocator.pthis, surface_out->Data.MemId, &hdl);
    DecodedFrame* frame = new DecodedFrame();
    frame->texture = (uint8_t*)hdl;
    frame->width = ctx->width;
    frame->height = ctx->height;
    return frame;
#else
    (void)decoder; (void)data; (void)length;
    return nullptr;
#endif
}

extern "C++" void mfx_DestroyDecoder(MfxDecoder* decoder) {
    if (!decoder) return;
#if defined(_WIN32) || defined(_WIN64)
    if (decoder->impl) {
        MfxDecContext* ctx = (MfxDecContext*)decoder->impl;
        if (pMFXVideoDECODE_Close) pMFXVideoDECODE_Close(ctx->session);
        mfxFrameAllocResponse resp = {};
        resp.mids = ctx->alloc_ctx.mids;
        resp.NumFrameActual = ctx->alloc_ctx.num;
        ctx->allocator.Free(ctx->allocator.pthis, &resp);
        if (pMFXClose) pMFXClose(ctx->session);
        delete ctx;
    }
#endif
    decoder->impl = nullptr;
    delete decoder;
}

extern "C++" int32_t mfx_GetWidth(MfxDecoder* decoder) {
    if (!decoder || !decoder->impl || !IsMfxAvailable()) return 0;
#if defined(_WIN32) || defined(_WIN64)
    return ((MfxDecContext*)decoder->impl)->width;
#else
    return 0;
#endif
}

extern "C++" int32_t mfx_GetHeight(MfxDecoder* decoder) {
    if (!decoder || !decoder->impl || !IsMfxAvailable()) return 0;
#if defined(_WIN32) || defined(_WIN64)
    return ((MfxDecContext*)decoder->impl)->height;
#else
    return 0;
#endif
}
