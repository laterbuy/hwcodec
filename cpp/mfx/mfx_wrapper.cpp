// Intel MediaSDK (MFX) C 包装层实现
// 只包装 SDK 的 C++ 接口调用，不包含业务逻辑
// 所有业务逻辑在 Rust 中实现

#include "mfx_wrapper.h"
#include "../common/common.h"
#include "../common/system.h"
#include "../common/platform/win/win_rust_ffi.h"
#include <mfxvideo.h>
#include <mfxvideo++.h>
#include <sample_defs.h>
#include <sample_utils.h>
#include <d3d11_allocator.h>
#include <d3d11.h>
#include <cstring>
#include <cstdint>

#ifdef __cplusplus
extern "C" {
#endif

// 简化的帧分配器回调
mfxStatus MFX_CDECL simple_getHDL(mfxHDL, mfxMemId mid, mfxHDL *handle) {
    mfxHDLPair *pair = (mfxHDLPair *)handle;
    pair->first = mid;
    pair->second = (mfxHDL)(UINT)0;
    return MFX_ERR_NONE;
}

// 全局帧分配器（简化版本）
static mfxFrameAllocator g_frameAllocator = {
    {}, NULL, NULL, NULL, NULL, simple_getHDL, NULL
};

// MFX Session 操作
int mfx_wrapper_session_init(void** session) {
    try {
        MFXVideoSession* sess = new MFXVideoSession();
        mfxInitParam mfxparams{};
        mfxIMPL impl = MFX_IMPL_HARDWARE_ANY | MFX_IMPL_VIA_D3D11;
        mfxparams.Implementation = impl;
        mfxparams.Version.Major = 1;
        mfxparams.Version.Minor = 0;
        mfxparams.GPUCopy = MFX_GPUCOPY_OFF;
        
        mfxStatus sts = sess->InitEx(mfxparams);
        if (sts != MFX_ERR_NONE) {
            delete sess;
            return -1;
        }
        *session = sess;
        return 0;
    } catch (...) {
        return -1;
    }
}

int mfx_wrapper_session_set_handle_d3d11(void* session, void* device) {
    try {
        MFXVideoSession* sess = static_cast<MFXVideoSession*>(session);
        mfxStatus sts = sess->SetHandle(MFX_HANDLE_D3D11_DEVICE, device);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int mfx_wrapper_session_set_frame_allocator(void* session, void* allocator) {
    try {
        MFXVideoSession* sess = static_cast<MFXVideoSession*>(session);
        mfxFrameAllocator* alloc = static_cast<mfxFrameAllocator*>(allocator);
        mfxStatus sts = sess->SetFrameAllocator(alloc);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

void mfx_wrapper_session_close(void* session) {
    if (session) {
        MFXVideoSession* sess = static_cast<MFXVideoSession*>(session);
        sess->Close();
        delete sess;
    }
}

// 编码器操作
int mfx_wrapper_create_encoder(void* session, void** encoder) {
    try {
        MFXVideoSession* sess = static_cast<MFXVideoSession*>(session);
        MFXVideoENCODE* enc = new MFXVideoENCODE(*sess);
        *encoder = enc;
        return 0;
    } catch (...) {
        return -1;
    }
}

void mfx_wrapper_encoder_close(void* encoder) {
    if (encoder) {
        MFXVideoENCODE* enc = static_cast<MFXVideoENCODE*>(encoder);
        enc->Close();
        delete enc;
    }
}

int mfx_wrapper_encoder_get_video_param(void* encoder, void* params) {
    try {
        MFXVideoENCODE* enc = static_cast<MFXVideoENCODE*>(encoder);
        mfxVideoParam* p = static_cast<mfxVideoParam*>(params);
        mfxStatus sts = enc->GetVideoParam(p);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int mfx_wrapper_encoder_reset(void* encoder, void* params) {
    try {
        MFXVideoENCODE* enc = static_cast<MFXVideoENCODE*>(encoder);
        mfxVideoParam* p = static_cast<mfxVideoParam*>(params);
        mfxStatus sts = enc->Reset(p);
        MSDK_IGNORE_MFX_STS(sts, MFX_WRN_INCOMPATIBLE_VIDEO_PARAM);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int mfx_wrapper_encoder_encode_frame_async(void* encoder, void* surface, void* bitstream, void* syncp) {
    try {
        MFXVideoENCODE* enc = static_cast<MFXVideoENCODE*>(encoder);
        mfxFrameSurface1* surf = static_cast<mfxFrameSurface1*>(surface);
        mfxBitstream* bs = static_cast<mfxBitstream*>(bitstream);
        mfxSyncPoint* sp = static_cast<mfxSyncPoint*>(syncp);
        
        mfxStatus sts = enc->EncodeFrameAsync(NULL, surf, bs, sp);
        
        if (sts == MFX_ERR_MORE_DATA) {
            return 1; // 需要更多输入
        }
        if (sts == MFX_ERR_MORE_SURFACE) {
            return 2; // 需要更多 Surface
        }
        if (sts == MFX_WRN_DEVICE_BUSY) {
            return 3; // 设备忙
        }
        if (sts == MFX_ERR_NONE) {
            return 0; // 成功
        }
        return -1; // 失败
    } catch (...) {
        return -1;
    }
}

int mfx_wrapper_sync_operation(void* session, void* syncp, uint32_t timeout) {
    try {
        MFXVideoSession* sess = static_cast<MFXVideoSession*>(session);
        mfxSyncPoint* sp = static_cast<mfxSyncPoint*>(syncp);
        mfxStatus sts = sess->SyncOperation(*sp, timeout);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

// 解码器操作
int mfx_wrapper_create_decoder(void* session, void** decoder) {
    try {
        MFXVideoSession* sess = static_cast<MFXVideoSession*>(session);
        MFXVideoDECODE* dec = new MFXVideoDECODE(*sess);
        *decoder = dec;
        return 0;
    } catch (...) {
        return -1;
    }
}

void mfx_wrapper_decoder_close(void* decoder) {
    if (decoder) {
        MFXVideoDECODE* dec = static_cast<MFXVideoDECODE*>(decoder);
        dec->Close();
        delete dec;
    }
}

int mfx_wrapper_decoder_query(void* decoder, void* params, void* caps) {
    try {
        MFXVideoDECODE* dec = static_cast<MFXVideoDECODE*>(decoder);
        mfxVideoParam* p = static_cast<mfxVideoParam*>(params);
        mfxVideoParam* c = static_cast<mfxVideoParam*>(caps);
        mfxStatus sts = dec->Query(p, c);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int mfx_wrapper_decoder_init(void* decoder, void* params) {
    try {
        MFXVideoDECODE* dec = static_cast<MFXVideoDECODE*>(decoder);
        mfxVideoParam* p = static_cast<mfxVideoParam*>(params);
        mfxStatus sts = dec->Init(p);
        MSDK_IGNORE_MFX_STS(sts, MFX_WRN_PARTIAL_ACCELERATION);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int mfx_wrapper_decoder_decode_frame_async(void* decoder, void* bitstream, void* surface_work, void* surface_out, void* syncp) {
    try {
        MFXVideoDECODE* dec = static_cast<MFXVideoDECODE*>(decoder);
        mfxBitstream* bs = static_cast<mfxBitstream*>(bitstream);
        mfxFrameSurface1* surf_work = static_cast<mfxFrameSurface1*>(surface_work);
        mfxFrameSurface1** surf_out = static_cast<mfxFrameSurface1**>(surface_out);
        mfxSyncPoint* sp = static_cast<mfxSyncPoint*>(syncp);
        
        mfxStatus sts = dec->DecodeFrameAsync(bs, surf_work, surf_out, sp);
        
        if (sts == MFX_ERR_MORE_DATA) {
            return 1; // 需要更多数据
        }
        if (sts == MFX_ERR_MORE_SURFACE) {
            return 2; // 需要更多 Surface
        }
        if (sts == MFX_WRN_DEVICE_BUSY) {
            return 3; // 设备忙
        }
        if (sts == MFX_ERR_NONE) {
            return 0; // 成功
        }
        return -1; // 失败
    } catch (...) {
        return -1;
    }
}

int mfx_wrapper_decoder_get_surface(void* decoder, void** surface) {
    // 这个函数实际上应该由调用者管理 Surface 池
    // 这里只是占位符
    return -1;
}

// Surface 操作
void* mfx_wrapper_surface_get_mem_id(void* surface) {
    try {
        mfxFrameSurface1* surf = static_cast<mfxFrameSurface1*>(surface);
        return surf->Data.MemId;
    } catch (...) {
        return nullptr;
    }
}

int mfx_wrapper_surface_get_info(void* surface, void* info) {
    try {
        mfxFrameSurface1* surf = static_cast<mfxFrameSurface1*>(surface);
        mfxFrameInfo* i = static_cast<mfxFrameInfo*>(info);
        *i = surf->Info;
        return 0;
    } catch (...) {
        return -1;
    }
}

// Bitstream 操作
void mfx_wrapper_bitstream_init(void* bitstream, void* data, uint32_t length) {
    mfxBitstream* bs = static_cast<mfxBitstream*>(bitstream);
    memset(bs, 0, sizeof(mfxBitstream));
    bs->Data = static_cast<mfxU8*>(data);
    bs->DataLength = length;
    bs->MaxLength = length;
    bs->DataFlag = MFX_BITSTREAM_COMPLETE_FRAME;
}

void* mfx_wrapper_bitstream_get_data(void* bitstream) {
    mfxBitstream* bs = static_cast<mfxBitstream*>(bitstream);
    return bs->Data + bs->DataOffset;
}

uint32_t mfx_wrapper_bitstream_get_length(void* bitstream) {
    mfxBitstream* bs = static_cast<mfxBitstream*>(bitstream);
    return bs->DataLength;
}

uint32_t mfx_wrapper_bitstream_get_frame_type(void* bitstream) {
    mfxBitstream* bs = static_cast<mfxBitstream*>(bitstream);
    return bs->FrameType;
}

// 帧分配器操作
int mfx_wrapper_create_d3d11_frame_allocator(void* device, void** allocator) {
    try {
        D3D11FrameAllocator* alloc = new D3D11FrameAllocator();
        D3D11AllocatorParams allocParams;
        allocParams.bUseSingleTexture = false; // important
        allocParams.pDevice = static_cast<ID3D11Device*>(device);
        allocParams.uncompressedResourceMiscFlags = 0;
        mfxStatus sts = alloc->Init(&allocParams);
        if (sts != MFX_ERR_NONE) {
            delete alloc;
            return -1;
        }
        *allocator = alloc;
        return 0;
    } catch (...) {
        return -1;
    }
}

int mfx_wrapper_allocator_alloc(void* allocator, void* request, void* response) {
    try {
        D3D11FrameAllocator* alloc = static_cast<D3D11FrameAllocator*>(allocator);
        mfxFrameAllocRequest* req = static_cast<mfxFrameAllocRequest*>(request);
        mfxFrameAllocResponse* resp = static_cast<mfxFrameAllocResponse*>(response);
        mfxStatus sts = alloc->AllocFrames(req, resp);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int mfx_wrapper_allocator_free(void* allocator, void* response) {
    try {
        D3D11FrameAllocator* alloc = static_cast<D3D11FrameAllocator*>(allocator);
        mfxFrameAllocResponse* resp = static_cast<mfxFrameAllocResponse*>(response);
        mfxStatus sts = alloc->FreeFrames(resp);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

void mfx_wrapper_allocator_release(void* allocator) {
    if (allocator) {
        D3D11FrameAllocator* alloc = static_cast<D3D11FrameAllocator*>(allocator);
        alloc->Close();
        delete alloc;
    }
}

// 获取全局帧分配器（简化版本）
void* mfx_wrapper_get_simple_frame_allocator() {
    return &g_frameAllocator;
}

// 高级接口：编码器参数设置
void* mfx_wrapper_create_encoder_params(
    int32_t codec_id,
    int32_t width,
    int32_t height,
    int32_t framerate,
    int32_t bitrate_kbps,
    int32_t gop
) {
    try {
        mfxVideoParam* params = new mfxVideoParam();
        memset(params, 0, sizeof(mfxVideoParam));
        
        // Basic
        params->mfx.CodecId = codec_id;
        params->mfx.BRCParamMultiplier = 0;
        
        // Frame Info
        params->mfx.FrameInfo.FrameRateExtN = framerate;
        params->mfx.FrameInfo.FrameRateExtD = 1;
        params->mfx.FrameInfo.FourCC = MFX_FOURCC_NV12;
        params->mfx.FrameInfo.ChromaFormat = MFX_CHROMAFORMAT_YUV420;
        params->mfx.FrameInfo.BitDepthLuma = 8;
        params->mfx.FrameInfo.BitDepthChroma = 8;
        params->mfx.FrameInfo.Shift = 0;
        params->mfx.FrameInfo.PicStruct = MFX_PICSTRUCT_PROGRESSIVE;
        params->mfx.FrameInfo.CropX = 0;
        params->mfx.FrameInfo.CropY = 0;
        params->mfx.FrameInfo.CropW = width;
        params->mfx.FrameInfo.CropH = height;
        params->mfx.FrameInfo.Width = MSDK_ALIGN16(width);
        params->mfx.FrameInfo.Height = MSDK_ALIGN16(height);
        
        // Encoding Options
        params->mfx.EncodedOrder = 0;
        params->IOPattern = MFX_IOPATTERN_IN_VIDEO_MEMORY;
        params->AsyncDepth = 1;
        params->mfx.GopRefDist = 1;
        params->mfx.GopPicSize = (gop > 0 && gop < 0xFFFF) ? gop : 0xFFFF;
        
        // Quality
        params->mfx.TargetUsage = MFX_TARGETUSAGE_BEST_SPEED;
        params->mfx.RateControlMethod = MFX_RATECONTROL_VBR;
        params->mfx.InitialDelayInKB = 0;
        params->mfx.BufferSizeInKB = 512;
        params->mfx.TargetKbps = bitrate_kbps;
        params->mfx.MaxKbps = bitrate_kbps;
        params->mfx.NumSlice = 1;
        params->mfx.NumRefFrame = 0;
        
        // Codec specific
        if (codec_id == MFX_CODEC_AVC) {
            params->mfx.CodecLevel = MFX_LEVEL_AVC_51;
            params->mfx.CodecProfile = MFX_PROFILE_AVC_MAIN;
        } else if (codec_id == MFX_CODEC_HEVC) {
            params->mfx.CodecLevel = MFX_LEVEL_HEVC_51;
            params->mfx.CodecProfile = MFX_PROFILE_HEVC_MAIN;
        }
        
        return params;
    } catch (...) {
        return nullptr;
    }
}

void mfx_wrapper_destroy_encoder_params(void* params) {
    if (params) {
        delete static_cast<mfxVideoParam*>(params);
    }
}

// 辅助函数：对齐宽度
int32_t mfx_wrapper_align16(int32_t value) {
    return MSDK_ALIGN16(value);
}

// 辅助函数：对齐高度（渐进式）
int32_t mfx_wrapper_align16_height(int32_t value) {
    return MSDK_ALIGN16(value);
}

// 辅助函数：获取空闲 Surface 索引
int32_t mfx_wrapper_get_free_surface_index(void* surfaces, int32_t surface_count) {
    try {
        mfxFrameSurface1* surf_array = static_cast<mfxFrameSurface1*>(surfaces);
        mfxU16 idx = GetFreeSurfaceIndex(surf_array, surface_count);
        if (idx >= surface_count) {
            return -1;
        }
        return idx;
    } catch (...) {
        return -1;
    }
}

// 高级接口：查询编码器并初始化
int mfx_wrapper_encoder_query_and_init(void* encoder, void* params) {
    try {
        MFXVideoENCODE* enc = static_cast<MFXVideoENCODE*>(encoder);
        mfxVideoParam* p = static_cast<mfxVideoParam*>(params);
        
        // Query (验证参数)
        mfxStatus sts = enc->Query(p, p);
        MSDK_IGNORE_MFX_STS(sts, MFX_WRN_INCOMPATIBLE_VIDEO_PARAM);
        if (sts != MFX_ERR_NONE) {
            return -1;
        }
        
        // Init
        sts = enc->Init(p);
        if (sts != MFX_ERR_NONE) {
            return -1;
        }
        
        // GetVideoParam (获取实际参数)
        sts = enc->GetVideoParam(p);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

// 高级接口：查询编码器 Surface 需求
int32_t mfx_wrapper_encoder_query_iosurf(void* encoder, void* params) {
    try {
        MFXVideoENCODE* enc = static_cast<MFXVideoENCODE*>(encoder);
        mfxVideoParam* p = static_cast<mfxVideoParam*>(params);
        mfxFrameAllocRequest request;
        memset(&request, 0, sizeof(request));
        
        mfxStatus sts = enc->QueryIOSurf(p, &request);
        if (sts != MFX_ERR_NONE) {
            return -1;
        }
        return request.NumFrameSuggested;
    } catch (...) {
        return -1;
    }
}

// 高级接口：创建 Bitstream 结构
void* mfx_wrapper_create_bitstream(uint32_t max_length) {
    try {
        mfxBitstream* bs = new mfxBitstream();
        memset(bs, 0, sizeof(mfxBitstream));
        bs->MaxLength = max_length;
        return bs;
    } catch (...) {
        return nullptr;
    }
}

// 释放 Bitstream 结构
void mfx_wrapper_destroy_bitstream(void* bitstream) {
    if (bitstream) {
        delete static_cast<mfxBitstream*>(bitstream);
    }
}

// 高级接口：创建 Surface 数组
void* mfx_wrapper_create_surface_array(int32_t count, void* frame_info) {
    try {
        mfxFrameInfo* info = static_cast<mfxFrameInfo*>(frame_info);
        mfxFrameSurface1* surfaces = new mfxFrameSurface1[count];
        for (int32_t i = 0; i < count; i++) {
            memset(&surfaces[i], 0, sizeof(mfxFrameSurface1));
            if (info) {
                surfaces[i].Info = *info;
            }
        }
        return surfaces;
    } catch (...) {
        return nullptr;
    }
}

// 高级接口：获取 Surface 数组中的指定 Surface
void* mfx_wrapper_get_surface_at(void* surfaces, int32_t index) {
    try {
        mfxFrameSurface1* surf_array = static_cast<mfxFrameSurface1*>(surfaces);
        return &surf_array[index];
    } catch (...) {
        return nullptr;
    }
}

// 释放 Surface 数组
void mfx_wrapper_destroy_surface_array(void* surfaces) {
    if (surfaces) {
        delete[] static_cast<mfxFrameSurface1*>(surfaces);
    }
}

// 高级接口：设置 Surface 的 MemId
void mfx_wrapper_surface_set_mem_id(void* surface, void* mem_id) {
    try {
        mfxFrameSurface1* surf = static_cast<mfxFrameSurface1*>(surface);
        surf->Data.MemId = mem_id;
    } catch (...) {
    }
}

// 高级接口：创建 SyncPoint
void* mfx_wrapper_create_syncpoint() {
    try {
        mfxSyncPoint* sp = new mfxSyncPoint();
        *sp = nullptr;
        return sp;
    } catch (...) {
        return nullptr;
    }
}

// 释放 SyncPoint
void mfx_wrapper_destroy_syncpoint(void* syncp) {
    if (syncp) {
        delete static_cast<mfxSyncPoint*>(syncp);
    }
}

// 高级接口：解码器从 Bitstream 头初始化解码参数
int mfx_wrapper_decoder_decode_header(void* decoder, void* bitstream, void* params) {
    try {
        MFXVideoDECODE* dec = static_cast<MFXVideoDECODE*>(decoder);
        mfxBitstream* bs = static_cast<mfxBitstream*>(bitstream);
        mfxVideoParam* p = static_cast<mfxVideoParam*>(params);
        
        mfxStatus sts = dec->DecodeHeader(bs, p);
        MSDK_IGNORE_MFX_STS(sts, MFX_WRN_PARTIAL_ACCELERATION);
        return (sts == MFX_ERR_NONE) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

// 高级接口：查询解码器 Surface 需求
int32_t mfx_wrapper_decoder_query_iosurf(void* decoder, void* params) {
    try {
        MFXVideoDECODE* dec = static_cast<MFXVideoDECODE*>(decoder);
        mfxVideoParam* p = static_cast<mfxVideoParam*>(params);
        mfxFrameAllocRequest request;
        memset(&request, 0, sizeof(request));
        
        mfxStatus sts = dec->QueryIOSurf(p, &request);
        MSDK_IGNORE_MFX_STS(sts, MFX_WRN_PARTIAL_ACCELERATION);
        if (sts != MFX_ERR_NONE) {
            return -1;
        }
        return request.NumFrameSuggested;
    } catch (...) {
        return -1;
    }
}

// 高级接口：创建解码器参数（基础参数）
void* mfx_wrapper_create_decoder_params(int32_t codec_id) {
    try {
        mfxVideoParam* params = new mfxVideoParam();
        memset(params, 0, sizeof(mfxVideoParam));
        
        params->mfx.CodecId = codec_id;
        params->IOPattern = MFX_IOPATTERN_OUT_VIDEO_MEMORY;
        params->AsyncDepth = 1;
        params->mfx.DecodedOrder = true;
        params->mfx.FrameInfo.FrameRateExtN = 30;
        params->mfx.FrameInfo.FrameRateExtD = 1;
        params->mfx.FrameInfo.AspectRatioW = 1;
        params->mfx.FrameInfo.AspectRatioH = 1;
        params->mfx.FrameInfo.FourCC = MFX_FOURCC_NV12;
        params->mfx.FrameInfo.ChromaFormat = MFX_CHROMAFORMAT_YUV420;
        
        return params;
    } catch (...) {
        return nullptr;
    }
}

// 释放解码器参数结构
void mfx_wrapper_destroy_decoder_params(void* params) {
    if (params) {
        delete static_cast<mfxVideoParam*>(params);
    }
}

// 高级接口：初始化解码器（从 Bitstream 头，分配 Surface）
int mfx_wrapper_decoder_initialize_from_bitstream(
    void* decoder,
    void* bitstream,
    void* params,
    void* allocator,
    void** surfaces,
    int32_t* surface_count
) {
    try {
        MFXVideoDECODE* dec = static_cast<MFXVideoDECODE*>(decoder);
        mfxBitstream* bs = static_cast<mfxBitstream*>(bitstream);
        mfxVideoParam* p = static_cast<mfxVideoParam*>(params);
        D3D11FrameAllocator* alloc = static_cast<D3D11FrameAllocator*>(allocator);
        
        // DecodeHeader
        mfxStatus sts = dec->DecodeHeader(bs, p);
        MSDK_IGNORE_MFX_STS(sts, MFX_WRN_PARTIAL_ACCELERATION);
        if (sts != MFX_ERR_NONE) {
            return -1;
        }
        
        // QueryIOSurf
        mfxFrameAllocRequest request;
        memset(&request, 0, sizeof(request));
        sts = dec->QueryIOSurf(p, &request);
        MSDK_IGNORE_MFX_STS(sts, MFX_WRN_PARTIAL_ACCELERATION);
        if (sts != MFX_ERR_NONE) {
            return -1;
        }
        
        // AllocFrames
        mfxFrameAllocResponse response;
        memset(&response, 0, sizeof(response));
        sts = alloc->AllocFrames(&request, &response);
        if (sts != MFX_ERR_NONE) {
            return -1;
        }
        
        // Create Surface array
        int32_t count = request.NumFrameSuggested;
        mfxFrameSurface1* surf_array = new mfxFrameSurface1[count];
        for (int32_t i = 0; i < count; i++) {
            memset(&surf_array[i], 0, sizeof(mfxFrameSurface1));
            surf_array[i].Info = p->mfx.FrameInfo;
            surf_array[i].Data.MemId = response.mids[i];
        }
        
        // Init decoder
        sts = dec->Init(p);
        MSDK_IGNORE_MFX_STS(sts, MFX_WRN_PARTIAL_ACCELERATION);
        if (sts != MFX_ERR_NONE) {
            delete[] surf_array;
            alloc->FreeFrames(&response);
            return -1;
        }
        
        *surfaces = surf_array;
        *surface_count = count;
        return 0;
    } catch (...) {
        return -1;
    }
}

#ifdef __cplusplus
}
#endif
