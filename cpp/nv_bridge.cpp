#include <cstdint>
#include <cstdlib>
#include <cstring>
#include "nv_bridge.h"

#if defined(_WIN32) || defined(_WIN64)
#include <windows.h>
#include <d3d11.h>
#endif

#if defined(_WIN32) || defined(_WIN64)
#include <ffnvcodec/nvEncodeAPI.h>
#endif
#include <ffnvcodec/dynlink_loader.h>
#include <cmath>

static bool IsNvidiaEncodeAvailable() {
#if defined(_WIN32) || defined(_WIN64)
    HMODULE h = LoadLibraryA("nvEncodeAPI64.dll");
    if (h) {
        FreeLibrary(h);
        return true;
    }
#endif
    return false;
}

static bool IsNvidiaDecodeAvailable() {
#if defined(_WIN32) || defined(_WIN64)
    HMODULE h = LoadLibraryA("nvcuvid.dll");
    if (h) {
        FreeLibrary(h);
        return true;
    }
#endif
    return false;
}

bool IsNvidiaDriverAvailable() {
    return IsNvidiaEncodeAvailable() && IsNvidiaDecodeAvailable();
}

extern "C++" bool nv_IsEncodeDriverAvailable() {
    return IsNvidiaEncodeAvailable();
}

extern "C++" bool nv_IsDecodeDriverAvailable() {
    return IsNvidiaDecodeAvailable();
}

// 解码使用运行时 dynlink（cuda/nvcuvid），不参与编译期链接
extern "C++" bool nv_IsDecodeImplemented() {
    return true;
}

// EncodedFrame/DecodedFrame are returned to Rust; define for allocation.
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

// NvEncoder: real implementation attempts NVENC API when driver is available.
// CreateEncoder tries to load nvEncodeAPI and open session; on success stores context in encoder->impl.
// EncodeFrame uses stored context to encode when possible; returns nullptr on failure or when not initialized.
// DestroyEncoder closes session and frees context.
struct NvEncContext {
    void* hEncoder;
    void* nvenc_dll;
    int32_t width;
    int32_t height;
    int32_t codec_id;
    int32_t bitrate;
    int32_t framerate;
    int32_t gop;
    bool initialized;
};

static void nv_destroy_encoder_impl(NvEncContext* ctx) {
    if (!ctx) return;
#if defined(_WIN32) || defined(_WIN64)
    if (ctx->hEncoder && ctx->nvenc_dll) {
        typedef NVENCSTATUS (NVENCAPI *DestroyEncoderFn)(void*);
        DestroyEncoderFn nvEncDestroyEncoder = (DestroyEncoderFn)GetProcAddress((HMODULE)ctx->nvenc_dll, "NvEncDestroyEncoder");
        if (nvEncDestroyEncoder) nvEncDestroyEncoder(ctx->hEncoder);
    }
    if (ctx->nvenc_dll) FreeLibrary((HMODULE)ctx->nvenc_dll);
#endif
    delete ctx;
}

extern "C++" NvEncoder* nv_CreateEncoder(uint8_t* device, int32_t width, int32_t height, int32_t codec_id, int32_t bitrate, int32_t framerate, int32_t gop) {
    if (!IsNvidiaEncodeAvailable() || !device || width <= 0 || height <= 0) return nullptr;
#if defined(_WIN32) || defined(_WIN64)
    HMODULE nvenc_dll = LoadLibraryA("nvEncodeAPI64.dll");
    if (!nvenc_dll) return nullptr;
    typedef NVENCSTATUS (NVENCAPI *CreateInstanceFn)(NV_ENCODE_API_FUNCTION_LIST*);
    CreateInstanceFn createInstance = (CreateInstanceFn)GetProcAddress(nvenc_dll, "NvEncodeAPICreateInstance");
    if (!createInstance) { FreeLibrary(nvenc_dll); return nullptr; }
    NV_ENCODE_API_FUNCTION_LIST nvenc = { NV_ENCODE_API_FUNCTION_LIST_VER };
    if (createInstance(&nvenc) != NV_ENC_SUCCESS) { FreeLibrary(nvenc_dll); return nullptr; }
    if (!nvenc.nvEncOpenEncodeSessionEx) { FreeLibrary(nvenc_dll); return nullptr; }
    NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS sessionParams = { NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS_VER };
    sessionParams.deviceType = NV_ENC_DEVICE_TYPE_DIRECTX;
    sessionParams.device = device;
    sessionParams.apiVersion = NVENCAPI_VERSION;
    void* hEncoder = nullptr;
    if (nvenc.nvEncOpenEncodeSessionEx(&sessionParams, &hEncoder) != NV_ENC_SUCCESS) { FreeLibrary(nvenc_dll); return nullptr; }
    NvEncContext* ctx = new NvEncContext();
    ctx->hEncoder = hEncoder;
    ctx->nvenc_dll = nvenc_dll;
    ctx->width = width;
    ctx->height = height;
    ctx->codec_id = codec_id;
    ctx->bitrate = bitrate;
    ctx->framerate = framerate;
    ctx->gop = gop;
    ctx->initialized = false;
    if (nvenc.nvEncInitializeEncoder) {
        GUID codecGuid = (codec_id == 1) ? NV_ENC_CODEC_HEVC_GUID : NV_ENC_CODEC_H264_GUID;
        NV_ENC_PRESET_CONFIG presetConfig = { NV_ENC_PRESET_CONFIG_VER, { NV_ENC_CONFIG_VER } };
        if (nvenc.nvEncGetEncodePresetConfig && nvenc.nvEncGetEncodePresetConfig(hEncoder, codecGuid, NV_ENC_PRESET_P4_GUID, &presetConfig) == NV_ENC_SUCCESS) {
            NV_ENC_INITIALIZE_PARAMS initParams = { NV_ENC_INITIALIZE_PARAMS_VER };
            initParams.encodeGUID = codecGuid;
            initParams.presetGUID = NV_ENC_PRESET_P4_GUID;
            initParams.encodeWidth = (uint32_t)width;
            initParams.encodeHeight = (uint32_t)height;
            initParams.darWidth = (uint32_t)width;
            initParams.darHeight = (uint32_t)height;
            initParams.frameRateNum = (uint32_t)(framerate > 0 ? framerate : 30);
            initParams.frameRateDen = 1;
            initParams.enablePTD = 1;
            initParams.encodeConfig = &presetConfig.presetCfg;
            initParams.encodeConfig->rcParams.rateControlMode = NV_ENC_PARAMS_RC_CBR;
            initParams.encodeConfig->rcParams.averageBitRate = (uint32_t)(bitrate * 1000);
            initParams.encodeConfig->rcParams.maxBitRate = (uint32_t)(bitrate * 1000);
            initParams.encodeConfig->gopLength = (gop > 0 && gop < (int32_t)0xffff) ? (uint32_t)gop : NVENC_INFINITE_GOPLENGTH;
            if (nvenc.nvEncInitializeEncoder(hEncoder, &initParams) == NV_ENC_SUCCESS)
                ctx->initialized = true;
        }
    }
    NvEncoder* enc = new NvEncoder();
    enc->impl = ctx;
    return enc;
#else
    (void)device; (void)width; (void)height; (void)codec_id; (void)bitrate; (void)framerate; (void)gop;
    return nullptr;
#endif
}

extern "C++" EncodedFrame* nv_EncodeFrame(NvEncoder* encoder, uint8_t* texture, int64_t timestamp) {
    if (!encoder || !encoder->impl || !IsNvidiaEncodeAvailable()) return nullptr;
    NvEncContext* ctx = (NvEncContext*)encoder->impl;
    if (!ctx->initialized || !texture) return nullptr;
#if defined(_WIN32) || defined(_WIN64)
    HMODULE nvenc_dll = (HMODULE)ctx->nvenc_dll;
    typedef NVENCSTATUS (NVENCAPI *RegisterResourceFn)(void*, NV_ENC_REGISTER_RESOURCE*);
    typedef NVENCSTATUS (NVENCAPI *EncodePictureFn)(void*, NV_ENC_PIC_PARAMS*);
    typedef NVENCSTATUS (NVENCAPI *LockBitstreamFn)(void*, NV_ENC_LOCK_BITSTREAM*);
    typedef NVENCSTATUS (NVENCAPI *UnlockBitstreamFn)(void*, NV_ENC_OUTPUT_PTR);
    RegisterResourceFn nvEncRegisterResource = (RegisterResourceFn)GetProcAddress(nvenc_dll, "NvEncRegisterResource");
    EncodePictureFn nvEncEncodePicture = (EncodePictureFn)GetProcAddress(nvenc_dll, "NvEncEncodePicture");
    LockBitstreamFn nvEncLockBitstream = (LockBitstreamFn)GetProcAddress(nvenc_dll, "NvEncLockBitstream");
    UnlockBitstreamFn nvEncUnlockBitstream = (UnlockBitstreamFn)GetProcAddress(nvenc_dll, "NvEncUnlockBitstream");
    typedef NVENCSTATUS (NVENCAPI *CreateBitstreamFn)(void*, NV_ENC_CREATE_BITSTREAM_BUFFER*);
    typedef NVENCSTATUS (NVENCAPI *DestroyBitstreamFn)(void*, NV_ENC_OUTPUT_PTR);
    CreateBitstreamFn nvEncCreateBitstreamBuffer = (CreateBitstreamFn)GetProcAddress(nvenc_dll, "NvEncCreateBitstreamBuffer");
    DestroyBitstreamFn nvEncDestroyBitstreamBuffer = (DestroyBitstreamFn)GetProcAddress(nvenc_dll, "NvEncDestroyBitstreamBuffer");
    if (!nvEncRegisterResource || !nvEncEncodePicture || !nvEncLockBitstream || !nvEncUnlockBitstream || !nvEncCreateBitstreamBuffer || !nvEncDestroyBitstreamBuffer) return nullptr;
    NV_ENC_CREATE_BITSTREAM_BUFFER createBs = { NV_ENC_CREATE_BITSTREAM_BUFFER_VER, 0, NV_ENC_MEMORY_HEAP_AUTOSELECT, 0 };
    if (nvEncCreateBitstreamBuffer(ctx->hEncoder, &createBs) != NV_ENC_SUCCESS) return nullptr;
    NV_ENC_OUTPUT_PTR outputBitstream = createBs.bitstreamBuffer;
    NV_ENC_REGISTER_RESOURCE regRes = { NV_ENC_REGISTER_RESOURCE_VER };
    regRes.resourceType = NV_ENC_INPUT_RESOURCE_TYPE_DIRECTX;
    regRes.resourceToRegister = texture;
    regRes.width = (uint32_t)ctx->width;
    regRes.height = (uint32_t)ctx->height;
    regRes.bufferFormat = NV_ENC_BUFFER_FORMAT_ARGB;
    if (nvEncRegisterResource(ctx->hEncoder, &regRes) != NV_ENC_SUCCESS) return nullptr;
    NV_ENC_REGISTERED_PTR registered = regRes.registeredResource;
    NV_ENC_PIC_PARAMS picParams = { NV_ENC_PIC_PARAMS_VER };
    picParams.inputBuffer = registered;
    picParams.bufferFmt = NV_ENC_BUFFER_FORMAT_ARGB;
    picParams.inputWidth = (uint32_t)ctx->width;
    picParams.inputHeight = (uint32_t)ctx->height;
    picParams.inputPitch = (uint32_t)(ctx->width * 4);
    picParams.outputBitstream = outputBitstream;
    picParams.encodePicFlags = 0;
    if (nvEncEncodePicture(ctx->hEncoder, &picParams) != NV_ENC_SUCCESS) {
        typedef NVENCSTATUS (NVENCAPI *UnregisterFn)(void*, NV_ENC_REGISTERED_PTR);
        UnregisterFn nvEncUnregister = (UnregisterFn)GetProcAddress(nvenc_dll, "NvEncUnregisterResource");
        if (nvEncUnregister) nvEncUnregister(ctx->hEncoder, registered);
        nvEncDestroyBitstreamBuffer(ctx->hEncoder, outputBitstream);
        return nullptr;
    }
    NV_ENC_LOCK_BITSTREAM lockBs = { NV_ENC_LOCK_BITSTREAM_VER };
    lockBs.outputBitstream = outputBitstream;
    if (nvEncLockBitstream(ctx->hEncoder, &lockBs) != NV_ENC_SUCCESS) {
        typedef NVENCSTATUS (NVENCAPI *UnregisterFn)(void*, NV_ENC_REGISTERED_PTR);
        UnregisterFn nvEncUnregister = (UnregisterFn)GetProcAddress(nvenc_dll, "NvEncUnregisterResource");
        if (nvEncUnregister) nvEncUnregister(ctx->hEncoder, registered);
        nvEncDestroyBitstreamBuffer(ctx->hEncoder, outputBitstream);
        return nullptr;
    }
    EncodedFrame* frame = new EncodedFrame();
    frame->size = (int32_t)lockBs.bitstreamSizeInBytes;
    frame->data = (uint8_t*)malloc((size_t)frame->size);
    if (frame->data && frame->size > 0) memcpy(frame->data, lockBs.bitstreamBufferPtr, (size_t)frame->size);
    frame->is_keyframe = (lockBs.pictureType == NV_ENC_PIC_TYPE_IDR || lockBs.pictureType == NV_ENC_PIC_TYPE_I);
    frame->timestamp = timestamp;
    if (lockBs.outputBitstream) nvEncUnlockBitstream(ctx->hEncoder, lockBs.outputBitstream);
    nvEncDestroyBitstreamBuffer(ctx->hEncoder, outputBitstream);
    typedef NVENCSTATUS (NVENCAPI *UnregisterFn)(void*, NV_ENC_REGISTERED_PTR);
    UnregisterFn nvEncUnregister = (UnregisterFn)GetProcAddress(nvenc_dll, "NvEncUnregisterResource");
    if (nvEncUnregister) nvEncUnregister(ctx->hEncoder, registered);
    return frame;
#else
    (void)texture; (void)timestamp;
    return nullptr;
#endif
}

extern "C++" void nv_FreeEncodedFrame(EncodedFrame* frame) {
    if (!frame) return;
    if (frame->data) { free(frame->data); frame->data = nullptr; }
    delete frame;
}

extern "C++" void nv_FreeDecodedFrame(DecodedFrame* frame) {
    if (frame) delete frame;
}

extern "C++" void nv_DestroyEncoder(NvEncoder* encoder) {
    if (!encoder) return;
    if (encoder->impl) nv_destroy_encoder_impl((NvEncContext*)encoder->impl);
    encoder->impl = nullptr;
    delete encoder;
}

extern "C++" void nv_SetBitrate(NvEncoder* encoder, int32_t bitrate) {
    if (encoder && encoder->impl) ((NvEncContext*)encoder->impl)->bitrate = bitrate;
}

extern "C++" void nv_SetFramerate(NvEncoder* encoder, int32_t framerate) {
    if (encoder && encoder->impl) ((NvEncContext*)encoder->impl)->framerate = framerate;
}

// NVDEC decode context: all CUDA/cuvid loaded at runtime via dynlink (no link-time dependency)
struct NvDecContext {
    CudaFunctions* cudl = nullptr;
    CuvidFunctions* cvdl = nullptr;
    CUcontext cuCtx = nullptr;
    CUdevice cuDevice = 0;
    CUvideoctxlock ctxLock = nullptr;
    CUstream stream = nullptr;
    CUvideoparser hParser = nullptr;
    CUvideodecoder hDecoder = nullptr;
#if defined(_WIN32) || defined(_WIN64)
    ID3D11Device* d3d11 = nullptr;
#endif
    unsigned int outWidth = 0;
    unsigned int outLumaHeight = 0;
    int outChromaHeight = 0;
    int outSurfaceHeight = 0;
    int nBPP = 1;
    int nNumChromaPlanes = 1;
    cudaVideoSurfaceFormat outFormat = cudaVideoSurfaceFormat_NV12;
    uint8_t* hostFrame = nullptr;
    size_t hostFrameSize = 0;
    size_t hostPitch = 0;
    bool frameReady = false;
};

static int CUDAAPI HandleVideoSequence(void* pUserData, CUVIDEOFORMAT* pVideoFormat) {
    NvDecContext* ctx = (NvDecContext*)pUserData;
    if (!ctx || !ctx->cudl || !ctx->cvdl) return 0;
    CUVIDDECODECAPS caps = {};
    caps.eCodecType = pVideoFormat->codec;
    caps.eChromaFormat = pVideoFormat->chroma_format;
    caps.nBitDepthMinus8 = pVideoFormat->bit_depth_luma_minus8;
    if (ctx->cvdl->cuvidGetDecoderCaps(&caps) != CUDA_SUCCESS || !caps.bIsSupported)
        return 0;
    if ((unsigned)pVideoFormat->coded_width > caps.nMaxWidth || (unsigned)pVideoFormat->coded_height > caps.nMaxHeight)
        return 0;
    int nDecodeSurface = pVideoFormat->min_num_decode_surfaces;
    if (nDecodeSurface < 1) nDecodeSurface = 4;
    cudaVideoSurfaceFormat outFmt = (pVideoFormat->bit_depth_luma_minus8 > 0) ? cudaVideoSurfaceFormat_P016 : cudaVideoSurfaceFormat_NV12;
    if (!(caps.nOutputFormatMask & (1 << outFmt))) outFmt = cudaVideoSurfaceFormat_NV12;
    unsigned int w = pVideoFormat->display_area.right - pVideoFormat->display_area.left;
    unsigned int h = pVideoFormat->display_area.bottom - pVideoFormat->display_area.top;
    if (w == 0) w = pVideoFormat->coded_width;
    if (h == 0) h = pVideoFormat->coded_height;
    int bpp = (pVideoFormat->bit_depth_luma_minus8 > 0) ? 2 : 1;
    float chromaFactor = (outFmt == cudaVideoSurfaceFormat_NV12 || outFmt == cudaVideoSurfaceFormat_P016) ? 0.5f : 1.0f;
    int numChromaPlanes = (outFmt == cudaVideoSurfaceFormat_YUV444 || outFmt == cudaVideoSurfaceFormat_YUV444_16Bit) ? 2 : 1;
    int chromaHeight = (int)std::ceil(h * chromaFactor);
    if (ctx->hDecoder) {
        ctx->cvdl->cuvidDestroyDecoder(ctx->hDecoder);
        ctx->hDecoder = nullptr;
    }
    CUVIDDECODECREATEINFO createInfo = {};
    createInfo.CodecType = pVideoFormat->codec;
    createInfo.ChromaFormat = pVideoFormat->chroma_format;
    createInfo.OutputFormat = outFmt;
    createInfo.bitDepthMinus8 = pVideoFormat->bit_depth_luma_minus8;
    createInfo.ulWidth = pVideoFormat->coded_width;
    createInfo.ulHeight = pVideoFormat->coded_height;
    createInfo.ulNumDecodeSurfaces = (unsigned)nDecodeSurface;
    createInfo.ulCreationFlags = cudaVideoCreate_PreferCUVID;
    createInfo.ulTargetWidth = pVideoFormat->coded_width;
    createInfo.ulTargetHeight = pVideoFormat->coded_height;
    createInfo.ulNumOutputSurfaces = 2;
    createInfo.vidLock = ctx->ctxLock;
    createInfo.DeinterlaceMode = pVideoFormat->progressive_sequence ? cudaVideoDeinterlaceMode_Weave : cudaVideoDeinterlaceMode_Adaptive;
    createInfo.display_area.left = (short)pVideoFormat->display_area.left;
    createInfo.display_area.top = (short)pVideoFormat->display_area.top;
    createInfo.display_area.right = (short)pVideoFormat->display_area.right;
    createInfo.display_area.bottom = (short)pVideoFormat->display_area.bottom;
    if (ctx->cudl->cuCtxPushCurrent(ctx->cuCtx) != CUDA_SUCCESS) return 0;
    CUresult r = ctx->cvdl->cuvidCreateDecoder(&ctx->hDecoder, &createInfo);
    ctx->cudl->cuCtxPopCurrent(nullptr);
    if (r != CUDA_SUCCESS) return 0;
    ctx->outWidth = (outFmt == cudaVideoSurfaceFormat_NV12 || outFmt == cudaVideoSurfaceFormat_P016) ? ((w + 1) & ~1u) : w;
    ctx->outLumaHeight = h;
    ctx->outChromaHeight = chromaHeight;
    ctx->outSurfaceHeight = (int)pVideoFormat->coded_height;
    ctx->nBPP = bpp;
    ctx->nNumChromaPlanes = numChromaPlanes;
    ctx->outFormat = outFmt;
    size_t frameSize = (size_t)ctx->outWidth * ctx->outLumaHeight * ctx->nBPP + (size_t)ctx->outWidth * ctx->outChromaHeight * ctx->nNumChromaPlanes * ctx->nBPP;
    if (ctx->hostFrame) { delete[] ctx->hostFrame; ctx->hostFrame = nullptr; }
    ctx->hostFrame = new (std::nothrow) uint8_t[frameSize];
    ctx->hostFrameSize = frameSize;
    ctx->hostPitch = (size_t)ctx->outWidth * ctx->nBPP;
    return nDecodeSurface;
}

static int CUDAAPI HandlePictureDecode(void* pUserData, CUVIDPICPARAMS* pPicParams) {
    NvDecContext* ctx = (NvDecContext*)pUserData;
    if (!ctx || !ctx->hDecoder || !ctx->cudl || !ctx->cvdl) return 0;
    if (ctx->cudl->cuCtxPushCurrent(ctx->cuCtx) != CUDA_SUCCESS) return 0;
    CUresult r = ctx->cvdl->cuvidDecodePicture(ctx->hDecoder, pPicParams);
    ctx->cudl->cuCtxPopCurrent(nullptr);
    return (r == CUDA_SUCCESS) ? 1 : 0;
}

static DecodedFrame* CreateD3D11FrameFromHostNV12(ID3D11Device* dev, ID3D11DeviceContext* imm, const uint8_t* host, int w, int h, size_t pitch) {
    if (!dev || !imm || !host || w <= 0 || h <= 0) return nullptr;
    UINT fullH = (UINT)(h + (h / 2));
    D3D11_TEXTURE2D_DESC desc = {};
    desc.Width = (UINT)w;
    desc.Height = fullH;
    desc.MipLevels = 1;
    desc.ArraySize = 1;
    desc.Format = (DXGI_FORMAT)0x3231564E; /* NV12 */
    desc.SampleDesc.Count = 1;
    desc.SampleDesc.Quality = 0;
    desc.Usage = D3D11_USAGE_DEFAULT;
    desc.BindFlags = D3D11_BIND_SHADER_RESOURCE;
    desc.CPUAccessFlags = 0;
    desc.MiscFlags = 0;
    ID3D11Texture2D* tex = nullptr;
    if (FAILED(dev->CreateTexture2D(&desc, nullptr, &tex)) || !tex) return nullptr;
    UINT row = (UINT)(pitch > 0 ? pitch : (size_t)w);
    imm->UpdateSubresource(tex, 0, nullptr, host, row, row * fullH);
    DecodedFrame* frame = new DecodedFrame();
    frame->texture = (uint8_t*)tex;
    frame->width = w;
    frame->height = h;
    return frame;
}

static int CUDAAPI HandlePictureDisplay(void* pUserData, CUVIDPARSERDISPINFO* pDispInfo) {
    NvDecContext* ctx = (NvDecContext*)pUserData;
    if (!ctx || !ctx->hDecoder || !ctx->hostFrame || !ctx->cudl || !ctx->cvdl) return 0;
    CUVIDPROCPARAMS procParams = {};
    procParams.progressive_frame = pDispInfo->progressive_frame;
    procParams.second_field = pDispInfo->repeat_first_field + 1;
    procParams.top_field_first = pDispInfo->top_field_first;
    procParams.unpaired_field = pDispInfo->repeat_first_field < 0;
    procParams.output_stream = ctx->stream;
    CUdeviceptr dpSrc = 0;
    unsigned int srcPitch = 0;
    if (ctx->cudl->cuCtxPushCurrent(ctx->cuCtx) != CUDA_SUCCESS) return 0;
    CUresult r = ctx->cvdl->cuvidMapVideoFrame(ctx->hDecoder, pDispInfo->picture_index, &dpSrc, &srcPitch, &procParams);
    if (r != CUDA_SUCCESS) { ctx->cudl->cuCtxPopCurrent(nullptr); return 0; }
    CUDA_MEMCPY2D m = {};
    m.srcMemoryType = CU_MEMORYTYPE_DEVICE;
    m.srcDevice = dpSrc;
    m.srcPitch = srcPitch;
    m.dstMemoryType = CU_MEMORYTYPE_HOST;
    m.dstHost = ctx->hostFrame;
    m.dstPitch = ctx->hostPitch;
    m.WidthInBytes = ctx->outWidth * ctx->nBPP;
    m.Height = ctx->outLumaHeight;
    ctx->cudl->cuMemcpy2DAsync(&m, ctx->stream);
    m.srcDevice = (CUdeviceptr)((uint8_t*)dpSrc + (size_t)srcPitch * ((ctx->outSurfaceHeight + 1) & ~1));
    m.dstHost = ctx->hostFrame + ctx->hostPitch * ctx->outLumaHeight;
    m.Height = ctx->outChromaHeight;
    ctx->cudl->cuMemcpy2DAsync(&m, ctx->stream);
    if (ctx->nNumChromaPlanes == 2) {
        m.srcDevice = (CUdeviceptr)((uint8_t*)dpSrc + (size_t)srcPitch * ((ctx->outSurfaceHeight + 1) & ~1) * 2);
        m.dstHost = ctx->hostFrame + ctx->hostPitch * ctx->outLumaHeight * 2;
        ctx->cudl->cuMemcpy2DAsync(&m, ctx->stream);
    }
    ctx->cudl->cuStreamSynchronize(ctx->stream);
    ctx->cvdl->cuvidUnmapVideoFrame(ctx->hDecoder, dpSrc);
    ctx->cudl->cuCtxPopCurrent(nullptr);
    ctx->frameReady = true;
    return 1;
}

extern "C++" NvDecoder* nv_CreateDecoder(uint8_t* device, int32_t codec_id) {
    if (!IsNvidiaDecodeAvailable() || !device) return nullptr;
    CudaFunctions* cudl = nullptr;
    CuvidFunctions* cvdl = nullptr;
    if (cuda_load_functions(&cudl, nullptr) != 0 || !cudl) return nullptr;
    if (cuvid_load_functions(&cvdl, nullptr) != 0 || !cvdl) { cuda_free_functions(&cudl); return nullptr; }
    if (cudl->cuInit(0) != CUDA_SUCCESS) { cuvid_free_functions(&cvdl); cuda_free_functions(&cudl); return nullptr; }
    CUdevice cuDevice = 0;
    if (cudl->cuDeviceGet(&cuDevice, 0) != CUDA_SUCCESS) { cuvid_free_functions(&cvdl); cuda_free_functions(&cudl); return nullptr; }
    CUcontext cuCtx = nullptr;
    if (cudl->cuCtxCreate(&cuCtx, 0, cuDevice) != CUDA_SUCCESS) { cuvid_free_functions(&cvdl); cuda_free_functions(&cudl); return nullptr; }
    CUvideoctxlock ctxLock = nullptr;
    if (cvdl->cuvidCtxLockCreate(&ctxLock, cuCtx) != CUDA_SUCCESS) { cudl->cuCtxDestroy(cuCtx); cuvid_free_functions(&cvdl); cuda_free_functions(&cudl); return nullptr; }
    CUstream stream = nullptr;
    if (cudl->cuStreamCreate(&stream, 0) != CUDA_SUCCESS) { cvdl->cuvidCtxLockDestroy(ctxLock); cudl->cuCtxDestroy(cuCtx); cuvid_free_functions(&cvdl); cuda_free_functions(&cudl); return nullptr; }
    NvDecContext* ctx = new (std::nothrow) NvDecContext();
    if (!ctx) { cudl->cuStreamDestroy(stream); cvdl->cuvidCtxLockDestroy(ctxLock); cudl->cuCtxDestroy(cuCtx); cuvid_free_functions(&cvdl); cuda_free_functions(&cudl); return nullptr; }
    ctx->cudl = cudl;
    ctx->cvdl = cvdl;
    ctx->cuCtx = cuCtx;
    ctx->cuDevice = cuDevice;
    ctx->ctxLock = ctxLock;
    ctx->stream = stream;
#if defined(_WIN32) || defined(_WIN64)
    ctx->d3d11 = (ID3D11Device*)device;
#endif
    cudaVideoCodec codec = (codec_id == 1) ? cudaVideoCodec_HEVC : cudaVideoCodec_H264;
    CUVIDPARSERPARAMS parserParams = {};
    parserParams.CodecType = codec;
    parserParams.ulMaxNumDecodeSurfaces = 1;
    parserParams.ulClockRate = 1000;
    parserParams.ulMaxDisplayDelay = 0;
    parserParams.pUserData = ctx;
    parserParams.pfnSequenceCallback = HandleVideoSequence;
    parserParams.pfnDecodePicture = HandlePictureDecode;
    parserParams.pfnDisplayPicture = HandlePictureDisplay;
    parserParams.pfnGetOperatingPoint = nullptr;
    parserParams.pfnGetSEIMsg = nullptr;
    if (cvdl->cuvidCreateVideoParser(&ctx->hParser, &parserParams) != CUDA_SUCCESS) {
        delete ctx;
        cudl->cuStreamDestroy(stream);
        cvdl->cuvidCtxLockDestroy(ctxLock);
        cudl->cuCtxDestroy(cuCtx);
        cuvid_free_functions(&cvdl);
        cuda_free_functions(&cudl);
        return nullptr;
    }
    struct NvDecoder* dec = new struct NvDecoder();
    dec->impl = ctx;
    return dec;
}

extern "C++" DecodedFrame* nv_DecodeFrame(NvDecoder* decoder, uint8_t* data, int32_t length) {
    if (!decoder || !decoder->impl) return nullptr;
    NvDecContext* ctx = (NvDecContext*)decoder->impl;
    if (!ctx->cvdl || !ctx->hParser) return nullptr;
    ctx->frameReady = false;
    CUVIDSOURCEDATAPACKET packet = {};
    packet.payload = data;
    packet.payload_size = (unsigned)(length > 0 ? length : 0);
    packet.flags = CUVID_PKT_TIMESTAMP;
    packet.timestamp = 0;
    if (!data || length <= 0) packet.flags |= CUVID_PKT_ENDOFSTREAM;
    if (ctx->cvdl->cuvidParseVideoData(ctx->hParser, &packet) != CUDA_SUCCESS) return nullptr;
#if defined(_WIN32) || defined(_WIN64)
    if (!ctx->frameReady || !ctx->hostFrame || !ctx->d3d11) return nullptr;
    ID3D11DeviceContext* imm = nullptr;
    ctx->d3d11->GetImmediateContext(&imm);
    DecodedFrame* frame = CreateD3D11FrameFromHostNV12(ctx->d3d11, imm, ctx->hostFrame, (int)ctx->outWidth, (int)ctx->outLumaHeight, ctx->hostPitch);
    if (imm) imm->Release();
    return frame;
#else
    return nullptr;
#endif
}

static void nv_dec_context_destroy(NvDecContext* ctx) {
    if (!ctx) return;
    if (ctx->hParser && ctx->cvdl) { ctx->cvdl->cuvidDestroyVideoParser(ctx->hParser); ctx->hParser = nullptr; }
    if (ctx->cuCtx && ctx->cudl) {
        ctx->cudl->cuCtxPushCurrent(ctx->cuCtx);
        if (ctx->hDecoder && ctx->cvdl) { ctx->cvdl->cuvidDestroyDecoder(ctx->hDecoder); ctx->hDecoder = nullptr; }
        if (ctx->stream) { ctx->cudl->cuStreamDestroy(ctx->stream); ctx->stream = nullptr; }
        ctx->cudl->cuCtxPopCurrent(nullptr);
        ctx->cudl->cuCtxDestroy(ctx->cuCtx);
        ctx->cuCtx = nullptr;
    }
    if (ctx->ctxLock && ctx->cvdl) { ctx->cvdl->cuvidCtxLockDestroy(ctx->ctxLock); ctx->ctxLock = nullptr; }
    if (ctx->hostFrame) { delete[] ctx->hostFrame; ctx->hostFrame = nullptr; }
    if (ctx->cvdl) { cuvid_free_functions(&ctx->cvdl); ctx->cvdl = nullptr; }
    if (ctx->cudl) { cuda_free_functions(&ctx->cudl); ctx->cudl = nullptr; }
    ctx->d3d11 = nullptr;
    delete ctx;
}

extern "C++" void nv_DestroyDecoder(NvDecoder* decoder) {
    if (!decoder) return;
    if (decoder->impl) nv_dec_context_destroy((NvDecContext*)decoder->impl);
    decoder->impl = nullptr;
    delete decoder;
}
