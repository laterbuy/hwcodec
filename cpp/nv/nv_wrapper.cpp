// NVIDIA Video Codec SDK C 包装层实现
// 只包装 SDK 的 C++ 接口调用，不包含业务逻辑
// 所有业务逻辑在 Rust 中实现

#include "nv_wrapper.h"
#include "../common/common.h"
#include "../common/system.h"
#include "../common/platform/win/win_rust_ffi.h"
#include <Samples/NvCodec/NvEncoder/NvEncoderD3D11.h>
#include <Samples/NvCodec/NvDecoder/NvDecoder.h>
#include <Samples/Utils/NvCodecUtils.h>
#include <dynlink_cuda.h>
#include <dynlink_loader.h>
#include <d3d11.h>
#include <dxgi.h>
#include <wrl/client.h>
#include <cstring>
#include <cstdint>
#include <vector>

#ifndef MAX_GOP
#define MAX_GOP 0x7FFFFFFF
#endif

using Microsoft::WRL::ComPtr;

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// 编码器操作
// ============================================================================

int nv_wrapper_create_encoder(
    void* cuda_dl,
    void* nvenc_dl,
    void* device,
    int32_t width,
    int32_t height,
    int32_t codec_id,
    int32_t bitrate_kbps,
    int32_t framerate,
    int32_t gop,
    void** encoder
) {
    try {
        CudaFunctions* cuda_dl_ptr = static_cast<CudaFunctions*>(cuda_dl);
        NvencFunctions* nvenc_dl_ptr = static_cast<NvencFunctions*>(nvenc_dl);
        ID3D11Device* d3d_device = static_cast<ID3D11Device*>(device);
        
        // 确定编解码器 GUID
        GUID guidCodec;
        if (codec_id == 0) {  // H264
            guidCodec = NV_ENC_CODEC_H264_GUID;
        } else if (codec_id == 1) {  // H265
            guidCodec = NV_ENC_CODEC_HEVC_GUID;
        } else {
            return -1;
        }
        
        // 创建编码器
        int nExtraOutputDelay = 0;
        NvEncoderD3D11* enc = new NvEncoderD3D11(
            cuda_dl_ptr, nvenc_dl_ptr, d3d_device,
            width, height, NV_ENC_BUFFER_FORMAT_ARGB,
            nExtraOutputDelay, false, false
        );
        
        // 初始化参数
        NV_ENC_INITIALIZE_PARAMS initializeParams = {0};
        NV_ENC_CONFIG encodeConfig = {0};
        memset(&initializeParams, 0, sizeof(initializeParams));
        memset(&encodeConfig, 0, sizeof(encodeConfig));
        initializeParams.encodeConfig = &encodeConfig;
        
        enc->CreateDefaultEncoderParams(
            &initializeParams, guidCodec,
            NV_ENC_PRESET_P3_GUID,
            NV_ENC_TUNING_INFO_LOW_LATENCY
        );
        
        // 配置参数
        initializeParams.encodeConfig->frameIntervalP = 1;
        initializeParams.encodeConfig->rcParams.lookaheadDepth = 0;
        initializeParams.encodeConfig->rcParams.averageBitRate = bitrate_kbps * 1000;
        initializeParams.frameRateNum = framerate;
        initializeParams.frameRateDen = 1;
        initializeParams.encodeConfig->gopLength = 
            (gop > 0 && gop < MAX_GOP) ? gop : NVENC_INFINITE_GOPLENGTH;
        initializeParams.encodeConfig->rcParams.rateControlMode = NV_ENC_PARAMS_RC_CBR;
        
        // 编解码器特定配置
        if (codec_id == 0) {  // H264
            NV_ENC_CONFIG_H264* h264 = &encodeConfig.encodeCodecConfig.h264Config;
            h264->sliceMode = 3;
            h264->sliceModeData = 1;
            h264->repeatSPSPPS = 1;
            h264->chromaFormatIDC = 1;
            h264->level = NV_ENC_LEVEL_AUTOSELECT;
            encodeConfig.profileGUID = NV_ENC_H264_PROFILE_MAIN_GUID;
        } else if (codec_id == 1) {  // H265
            NV_ENC_CONFIG_HEVC* hevc = &encodeConfig.encodeCodecConfig.hevcConfig;
            hevc->sliceMode = 3;
            hevc->sliceModeData = 1;
            hevc->repeatSPSPPS = 1;
            hevc->chromaFormatIDC = 1;
            hevc->level = NV_ENC_LEVEL_AUTOSELECT;
            hevc->outputPictureTimingSEI = 1;
            hevc->tier = NV_ENC_TIER_HEVC_MAIN;
            encodeConfig.profileGUID = NV_ENC_HEVC_PROFILE_MAIN_GUID;
        }
        
        // 创建编码器
        enc->CreateEncoder(&initializeParams);
        
        *encoder = enc;
        return 0;
    } catch (...) {
        return -1;
    }
}

void nv_wrapper_destroy_encoder(void* encoder) {
    if (encoder) {
        try {
            NvEncoderD3D11* enc = static_cast<NvEncoderD3D11*>(encoder);
            enc->DestroyEncoder();
            delete enc;
        } catch (...) {
        }
    }
}

void* nv_wrapper_encoder_get_next_input_frame(void* encoder) {
    try {
        NvEncoderD3D11* enc = static_cast<NvEncoderD3D11*>(encoder);
        const NvEncInputFrame* input_frame = enc->GetNextInputFrame();
        if (input_frame) {
            return const_cast<void*>(static_cast<const void*>(input_frame->inputPtr));
        }
        return nullptr;
    } catch (...) {
        return nullptr;
    }
}

int nv_wrapper_encoder_encode_frame(
    void* encoder,
    void* input_texture,
    int64_t timestamp,
    void* packet_data,
    uint32_t* packet_size,
    uint32_t* picture_type
) {
    try {
        NvEncoderD3D11* enc = static_cast<NvEncoderD3D11*>(encoder);
        
        // 获取输入帧
        const NvEncInputFrame* input_frame = enc->GetNextInputFrame();
        if (!input_frame) {
            return -1;
        }
        
        // 注意：纹理复制应该在 Rust 中通过 NativeDevice 的上下文完成
        // 这里不进行复制，由调用者负责
        // input_texture 参数暂时不使用，实际复制在 Rust 中完成
        (void)input_texture;
        
        // 编码
        std::vector<NvPacket> vPacket;
        NV_ENC_PIC_PARAMS picParams = {0};
        picParams.inputTimeStamp = timestamp;
        enc->EncodeFrame(vPacket, &picParams);
        
        // 处理输出
        if (vPacket.empty()) {
            return -1;
        }
        
        // 取第一个数据包
        const NvPacket& packet = vPacket[0];
        if (packet.data.size() > *packet_size) {
            return -1;  // 缓冲区太小
        }
        
        memcpy(packet_data, packet.data.data(), packet.data.size());
        *packet_size = static_cast<uint32_t>(packet.data.size());
        
        // 确定图片类型
        if (packet.pictureType == NV_ENC_PIC_TYPE_IDR || 
            packet.pictureType == NV_ENC_PIC_TYPE_I) {
            *picture_type = 1;  // 关键帧
        } else {
            *picture_type = 0;  // 非关键帧
        }
        
        return 0;
    } catch (...) {
        return -1;
    }
}

int nv_wrapper_encoder_reconfigure(
    void* encoder,
    int32_t bitrate_kbps,
    int32_t framerate
) {
    try {
        NvEncoderD3D11* enc = static_cast<NvEncoderD3D11*>(encoder);
        
        NV_ENC_CONFIG sEncodeConfig = {0};
        NV_ENC_INITIALIZE_PARAMS sInitializeParams = {0};
        sInitializeParams.encodeConfig = &sEncodeConfig;
        enc->GetInitializeParams(&sInitializeParams);
        
        NV_ENC_RECONFIGURE_PARAMS params = {0};
        params.version = NV_ENC_RECONFIGURE_PARAMS_VER;
        params.reInitEncodeParams = sInitializeParams;
        
        if (bitrate_kbps > 0) {
            params.reInitEncodeParams.encodeConfig->rcParams.averageBitRate = bitrate_kbps * 1000;
        }
        if (framerate > 0) {
            params.reInitEncodeParams.frameRateNum = framerate;
            params.reInitEncodeParams.frameRateDen = 1;
        }
        
        if (enc->Reconfigure(&params)) {
            return 0;
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

// ============================================================================
// 解码器操作
// ============================================================================

int nv_wrapper_create_decoder(
    void* cuda_dl,
    void* cuvid_dl,
    void* cu_context,
    int32_t codec_id,
    void** decoder
) {
    try {
        CudaFunctions* cuda_dl_ptr = static_cast<CudaFunctions*>(cuda_dl);
        CuvidFunctions* cuvid_dl_ptr = static_cast<CuvidFunctions*>(cuvid_dl);
        CUcontext cu_ctx = static_cast<CUcontext>(cu_context);
        
        // 确定编解码器 ID
        cudaVideoCodec cudaCodecID;
        if (codec_id == 0) {  // H264
            cudaCodecID = cudaVideoCodec_H264;
        } else if (codec_id == 1) {  // H265
            cudaCodecID = cudaVideoCodec_HEVC;
        } else {
            return -1;
        }
        
        bool bUseDeviceFrame = true;
        bool bLowLatency = true;
        bool bDeviceFramePitched = false;
        
        NvDecoder* dec = new NvDecoder(
            cuda_dl_ptr, cuvid_dl_ptr, cu_ctx,
            bUseDeviceFrame, cudaCodecID,
            bLowLatency, bDeviceFramePitched
        );
        
        *decoder = dec;
        return 0;
    } catch (...) {
        return -1;
    }
}

void nv_wrapper_destroy_decoder(void* decoder) {
    if (decoder) {
        try {
            NvDecoder* dec = static_cast<NvDecoder*>(decoder);
            delete dec;
        } catch (...) {
        }
    }
}

int nv_wrapper_decoder_decode(
    void* decoder,
    const uint8_t* data,
    int32_t length,
    uint32_t flags
) {
    try {
        NvDecoder* dec = static_cast<NvDecoder*>(decoder);
        int nFrameReturned = dec->Decode(data, length, flags);
        return nFrameReturned;
    } catch (...) {
        return -1;
    }
}

void* nv_wrapper_decoder_get_frame(void* decoder) {
    try {
        NvDecoder* dec = static_cast<NvDecoder*>(decoder);
        uint8_t* frame = dec->GetFrame();
        return frame;
    } catch (...) {
        return nullptr;
    }
}

int32_t nv_wrapper_decoder_get_width(void* decoder) {
    try {
        NvDecoder* dec = static_cast<NvDecoder*>(decoder);
        return static_cast<int32_t>(dec->GetWidth());
    } catch (...) {
        return -1;
    }
}

int32_t nv_wrapper_decoder_get_height(void* decoder) {
    try {
        NvDecoder* dec = static_cast<NvDecoder*>(decoder);
        return static_cast<int32_t>(dec->GetHeight());
    } catch (...) {
        return -1;
    }
}

int32_t nv_wrapper_decoder_get_chroma_height(void* decoder) {
    try {
        NvDecoder* dec = static_cast<NvDecoder*>(decoder);
        return static_cast<int32_t>(dec->GetChromaHeight());
    } catch (...) {
        return -1;
    }
}

// ============================================================================
// CUDA 操作
// ============================================================================

int nv_wrapper_load_encoder_driver(void** cuda_dl, void** nvenc_dl) {
    try {
        CudaFunctions* cuda_ptr = nullptr;
        NvencFunctions* nvenc_ptr = nullptr;
        
        if (cuda_load_functions(&cuda_ptr, NULL) < 0) {
            return -1;
        }
        if (nvenc_load_functions(&nvenc_ptr, NULL) < 0) {
            cuda_free_functions(&cuda_ptr);
            return -1;
        }
        
        *cuda_dl = cuda_ptr;
        *nvenc_dl = nvenc_ptr;
        return 0;
    } catch (...) {
        return -1;
    }
}

void nv_wrapper_free_encoder_driver(void** cuda_dl, void** nvenc_dl) {
    if (*nvenc_dl) {
        nvenc_free_functions(reinterpret_cast<NvencFunctions**>(nvenc_dl));
        *nvenc_dl = nullptr;
    }
    if (*cuda_dl) {
        cuda_free_functions(reinterpret_cast<CudaFunctions**>(cuda_dl));
        *cuda_dl = nullptr;
    }
}

int nv_wrapper_load_decoder_driver(void** cuda_dl, void** cuvid_dl) {
    try {
        CudaFunctions* cuda_ptr = nullptr;
        CuvidFunctions* cuvid_ptr = nullptr;
        
        if (cuda_load_functions(&cuda_ptr, NULL) < 0) {
            return -1;
        }
        if (cuvid_load_functions(&cuvid_ptr, NULL) < 0) {
            cuda_free_functions(&cuda_ptr);
            return -1;
        }
        
        *cuda_dl = cuda_ptr;
        *cuvid_dl = cuvid_ptr;
        return 0;
    } catch (...) {
        return -1;
    }
}

void nv_wrapper_free_decoder_driver(void** cuda_dl, void** cuvid_dl) {
    if (*cuvid_dl) {
        cuvid_free_functions(reinterpret_cast<CuvidFunctions**>(cuvid_dl));
        *cuvid_dl = nullptr;
    }
    if (*cuda_dl) {
        cuda_free_functions(reinterpret_cast<CudaFunctions**>(cuda_dl));
        *cuda_dl = nullptr;
    }
}

int nv_wrapper_cuda_init(void* cuda_dl) {
    try {
        CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
        if (cuda->cuInit(0) == 0) {
            return 0;
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

int nv_wrapper_cuda_get_device_from_d3d11(void* cuda_dl, void* adapter, uint32_t* cu_device) {
    try {
        CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
        IDXGIAdapter* dxgi_adapter = static_cast<IDXGIAdapter*>(adapter);
        CUdevice device = 0;
        if (cuda->cuD3D11GetDevice(&device, dxgi_adapter) == 0) {
            *cu_device = device;
            return 0;
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

int nv_wrapper_cuda_create_context(void* cuda_dl, uint32_t cu_device, void** cu_context) {
    try {
        CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
        CUcontext ctx = nullptr;
        if (cuda->cuCtxCreate(&ctx, 0, cu_device) == 0) {
            *cu_context = ctx;
            return 0;
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

void nv_wrapper_cuda_destroy_context(void* cuda_dl, void* cu_context) {
    if (cuda_dl && cu_context) {
        try {
            CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
            CUcontext ctx = static_cast<CUcontext>(cu_context);
            cuda->cuCtxDestroy(ctx);
        } catch (...) {
        }
    }
}

int nv_wrapper_cuda_push_context(void* cuda_dl, void* cu_context) {
    try {
        CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
        CUcontext ctx = static_cast<CUcontext>(cu_context);
        if (cuda->cuCtxPushCurrent(ctx) == 0) {
            return 0;
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

int nv_wrapper_cuda_pop_context(void* cuda_dl) {
    try {
        CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
        if (cuda->cuCtxPopCurrent(NULL) == 0) {
            return 0;
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

// ============================================================================
// 纹理操作（用于解码器）
// ============================================================================

int nv_wrapper_cuda_register_texture(void* cuda_dl, void* texture, void** cu_resource) {
    try {
        CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
        ID3D11Texture2D* tex = static_cast<ID3D11Texture2D*>(texture);
        CUgraphicsResource res = nullptr;
        if (cuda->cuGraphicsD3D11RegisterResource(
                &res, tex, CU_GRAPHICS_REGISTER_FLAGS_NONE) == 0) {
            if (cuda->cuGraphicsResourceSetMapFlags(
                    res, CU_GRAPHICS_REGISTER_FLAGS_WRITE_DISCARD) == 0) {
                *cu_resource = res;
                return 0;
            }
            cuda->cuGraphicsUnregisterResource(res);
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

void nv_wrapper_cuda_unregister_texture(void* cuda_dl, void* cu_resource) {
    if (cuda_dl && cu_resource) {
        try {
            CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
            CUgraphicsResource res = static_cast<CUgraphicsResource>(cu_resource);
            cuda->cuGraphicsUnregisterResource(res);
        } catch (...) {
        }
    }
}

int nv_wrapper_cuda_map_resource(void* cuda_dl, void* cu_resource) {
    try {
        CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
        CUgraphicsResource res = static_cast<CUgraphicsResource>(cu_resource);
        if (cuda->cuGraphicsMapResources(1, &res, 0) == 0) {
            return 0;
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

int nv_wrapper_cuda_unmap_resource(void* cuda_dl, void* cu_resource) {
    try {
        CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
        CUgraphicsResource res = static_cast<CUgraphicsResource>(cu_resource);
        if (cuda->cuGraphicsUnmapResources(1, &res, 0) == 0) {
            return 0;
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

void* nv_wrapper_cuda_get_mapped_array(void* cuda_dl, void* cu_resource) {
    try {
        CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
        CUgraphicsResource res = static_cast<CUgraphicsResource>(cu_resource);
        CUarray array = nullptr;
        if (cuda->cuGraphicsSubResourceGetMappedArray(&array, res, 0, 0) == 0) {
            return array;
        }
        return nullptr;
    } catch (...) {
        return nullptr;
    }
}

int nv_wrapper_cuda_memcpy_device_to_array(
    void* cuda_dl,
    void* dst_array,
    const void* src_device,
    uint32_t width,
    uint32_t height,
    uint32_t src_pitch
) {
    try {
        CudaFunctions* cuda = static_cast<CudaFunctions*>(cuda_dl);
        CUarray dst = static_cast<CUarray>(dst_array);
        CUdeviceptr src = reinterpret_cast<CUdeviceptr>(const_cast<void*>(src_device));
        
        CUDA_MEMCPY2D m = {0};
        m.srcMemoryType = CU_MEMORYTYPE_DEVICE;
        m.srcDevice = src;
        m.srcPitch = src_pitch;
        m.dstMemoryType = CU_MEMORYTYPE_ARRAY;
        m.dstArray = dst;
        m.WidthInBytes = width;
        m.Height = height;
        
        if (cuda->cuMemcpy2D(&m) == 0) {
            return 0;
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

#ifdef __cplusplus
}
#endif
