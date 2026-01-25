#include <public/common/AMFFactory.h>
#include <public/common/AMFSTL.h>
#include <public/common/Thread.h>
#include <public/common/TraceAdapter.h>
#include <public/include/components/VideoEncoderAV1.h>
#include <public/include/components/VideoEncoderHEVC.h>
#include <public/include/components/VideoEncoderVCE.h>
#include <public/include/components/VideoConverter.h>
#include <public/include/core/Platform.h>
#include <stdio.h>

#include <cstring>
#include <iostream>
#include <math.h>

#include "callback.h"
#include "common.h"
#include "system.h"
#include "util.h"
#include "platform/win/win_rust_ffi.h"

#define LOG_MODULE "AMFENC"
#include "log.h"

// ============================================================================
// NOTE: Windows Platform-Specific Code - PARTIALLY REPLACED
// ============================================================================
// This file contains Windows-specific AMF SDK calls:
//   - AMF_MEMORY_DX11: DirectX 11 memory type (Windows only)
//   - InitDX11(): Initialize AMF with D3D11 device (Windows only)
//   - CreateSurfaceFromDX11Native(): Create AMF surface from D3D11 texture (Windows only)
//
// REPLACEMENT STATUS:
//   ✅ Windows platform management (device creation, adapter enumeration) → Rust (src/platform/win/)
//   ✅ Device access → Rust FFI (hwcodec_adapters_get_adapter_device)
//   ⚠️  AMF SDK API calls → MUST REMAIN IN C++ (AMF SDK is C++ library)
//
// WHY AMF SDK CALLS CANNOT BE REPLACED:
//   1. AMF SDK is a C++ library with C++ interfaces (smart pointers, COM-like)
//   2. AMF SDK requires direct C++ object manipulation (AMFFactory, AMFContext, etc.)
//   3. Creating Rust bindings for entire AMF SDK would be extremely complex
//   4. The C++ code here is minimal - just AMF SDK wrapper calls
//
// All Windows platform management (device creation, adapter enumeration) is handled
// by Rust implementations in src/platform/win/ and exposed via FFI.
// This file now only contains AMF SDK wrapper code, not Windows platform code.
// ============================================================================

#define AMF_FACILITY L"AMFEncoder"
#define MILLISEC_TIME 10000

namespace {

#define AMF_CHECK_RETURN(res, msg)                                             \
  if (res != AMF_OK) {                                                         \
    LOG_ERROR(std::string(msg) + ", result code: " + std::to_string(int(res)));             \
    return res;                                                                \
  }

/** Encoder output packet */
struct encoder_packet {
  uint8_t *data; /**< Packet data */
  size_t size;   /**< Packet size */

  int64_t pts; /**< Presentation timestamp */
  int64_t dts; /**< Decode timestamp */

  int32_t timebase_num; /**< Timebase numerator */
  int32_t timebase_den; /**< Timebase denominator */

  bool keyframe; /**< Is a keyframe */

  /* ---------------------------------------------------------------- */
  /* Internal video variables (will be parsed automatically) */

  /* DTS in microseconds */
  int64_t dts_usec;

  /* System DTS in microseconds */
  int64_t sys_dts_usec;
};

class AMFEncoder {

public:
  DataFormat dataFormat_;
  amf::AMFComponentPtr AMFEncoder_ = NULL;
  amf::AMFContextPtr AMFContext_ = NULL;

private:
  // system
  void *handle_;
  // AMF Internals
  AMFFactoryHelper AMFFactory_;
  amf::AMF_MEMORY_TYPE AMFMemoryType_;
  amf::AMF_SURFACE_FORMAT AMFSurfaceFormat_ = amf::AMF_SURFACE_BGRA;
  std::pair<int32_t, int32_t> resolution_;
  amf_wstring codec_;
  // const
  AMF_COLOR_BIT_DEPTH_ENUM eDepth_ = AMF_COLOR_BIT_DEPTH_8;
  int query_timeout_ = ENCODE_TIMEOUT_MS;
  int32_t bitRateIn_;
  int32_t frameRate_;
  int32_t gop_;
  bool enable4K_ = false;
  bool full_range_ = false;
  bool bt709_ = false;

  // Converter for HEVC (BGRA -> NV12)
  amf::AMFComponentPtr AMFConverter_ = NULL;

  // Buffers
  std::vector<uint8_t> packetDataBuffer_;

public:
  AMFEncoder(void *handle, amf::AMF_MEMORY_TYPE memoryType, amf_wstring codec,
             DataFormat dataFormat, int32_t width, int32_t height,
             int32_t bitrate, int32_t framerate, int32_t gop) {
    handle_ = handle;
    dataFormat_ = dataFormat;
    AMFMemoryType_ = memoryType;
    resolution_ = std::make_pair(width, height);
    codec_ = codec;
    bitRateIn_ = bitrate;
    frameRate_ = framerate;
    gop_ = (gop > 0 && gop < MAX_GOP) ? gop : MAX_GOP;
    enable4K_ = width > 1920 && height > 1080;
  }

  ~AMFEncoder() {}

  AMF_RESULT encode(void *tex, EncodeCallback callback, void *obj, int64_t ms) {
    amf::AMFSurfacePtr surface = NULL;
    amf::AMFComputeSyncPointPtr pSyncPoint = NULL;
    AMF_RESULT res;
    bool encoded = false;

    switch (AMFMemoryType_) {
    case amf::AMF_MEMORY_DX11:
      // NOTE: Windows-specific AMF surface creation using DirectX 11
      // This is the only Windows platform-specific code in this method - required by AMF SDK
      // https://github.com/GPUOpen-LibrariesAndSDKs/AMF/issues/280
      // AMF will not copy the surface during the CreateSurfaceFromDX11Native call
      res = AMFContext_->CreateSurfaceFromDX11Native(tex, &surface, NULL);
      AMF_CHECK_RETURN(res, "CreateSurfaceFromDX11Native failed");
      {
        amf::AMFDataPtr data1;
        surface->Duplicate(surface->GetMemoryType(), &data1);
        surface = amf::AMFSurfacePtr(data1);
      }
      break;
    default:
      LOG_ERROR(std::string("Unsupported memory type"));
      return AMF_NOT_IMPLEMENTED;
      break;
    }
    surface->SetPts(ms * AMF_MILLISECOND);

    // For HEVC encoder, convert BGRA to NV12
    if (codec_ == amf_wstring(AMFVideoEncoder_HEVC)) {
      if (!AMFConverter_) {
        res = AMFFactory_.GetFactory()->CreateComponent(
            AMFContext_, AMFVideoConverter, &AMFConverter_);
        AMF_CHECK_RETURN(res, "CreateConverter failed");
        res = AMFConverter_->SetProperty(AMF_VIDEO_CONVERTER_MEMORY_TYPE,
                                         AMFMemoryType_);
        AMF_CHECK_RETURN(res, "SetProperty AMF_VIDEO_CONVERTER_MEMORY_TYPE failed");
        res = AMFConverter_->SetProperty(AMF_VIDEO_CONVERTER_OUTPUT_FORMAT,
                                         amf::AMF_SURFACE_NV12);
        AMF_CHECK_RETURN(res, "SetProperty AMF_VIDEO_CONVERTER_OUTPUT_FORMAT failed");
        res = AMFConverter_->SetProperty(AMF_VIDEO_CONVERTER_OUTPUT_SIZE,
                                         ::AMFConstructSize(resolution_.first, resolution_.second));
        AMF_CHECK_RETURN(res, "SetProperty AMF_VIDEO_CONVERTER_OUTPUT_SIZE failed");
        res = AMFConverter_->Init(amf::AMF_SURFACE_BGRA, resolution_.first, resolution_.second);
        AMF_CHECK_RETURN(res, "Init converter failed");
      }
      amf::AMFDataPtr convertData;
      res = AMFConverter_->SubmitInput(surface);
      AMF_CHECK_RETURN(res, "Converter SubmitInput failed");
      res = AMFConverter_->QueryOutput(&convertData);
      AMF_CHECK_RETURN(res, "Converter QueryOutput failed");
      surface = amf::AMFSurfacePtr(convertData);
      if (!surface) {
        return AMF_FAIL;
      }
    }

    res = AMFEncoder_->SubmitInput(surface);
    AMF_CHECK_RETURN(res, "SubmitInput failed");

    amf::AMFDataPtr data = NULL;
    res = AMFEncoder_->QueryOutput(&data);
    if (res == AMF_OK && data != NULL) {
      struct encoder_packet packet;
      PacketKeyframe(data, &packet);
      amf::AMFBufferPtr pBuffer = amf::AMFBufferPtr(data);
      packet.size = pBuffer->GetSize();
      if (packet.size > 0) {
        if (packetDataBuffer_.size() < packet.size) {
          size_t newBufferSize = (size_t)exp2(ceil(log2((double)packet.size)));
          packetDataBuffer_.resize(newBufferSize);
        }
        packet.data = packetDataBuffer_.data();
        std::memcpy(packet.data, pBuffer->GetNative(), packet.size);
        if (callback)
          callback(packet.data, static_cast<int32_t>(packet.size), packet.keyframe, obj, ms);
        encoded = true;
      }
      pBuffer = NULL;
    }
    data = NULL;
    pSyncPoint = NULL;
    surface = NULL;
    return encoded ? AMF_OK : AMF_FAIL;
  }

  AMF_RESULT destroy() {
    if (AMFConverter_) {
      AMFConverter_->Terminate();
      AMFConverter_ = NULL;
    }
    if (AMFEncoder_) {
      AMFEncoder_->Terminate();
      AMFEncoder_ = NULL;
    }
    if (AMFContext_) {
      AMFContext_->Terminate();
      AMFContext_ = NULL; // AMFContext_ is the last
    }
    AMFFactory_.Terminate();
    return AMF_OK;
  }

  AMF_RESULT test() {
    try {
      AMF_RESULT res = AMF_OK;
      amf::AMFSurfacePtr surface = nullptr;
      res = AMFContext_->AllocSurface(AMFMemoryType_, AMFSurfaceFormat_,
                                      resolution_.first, resolution_.second,
                                      &surface);
      if (res != AMF_OK) {
        return AMF_FAIL;
      }
      if (surface->GetPlanesCount() < 1)
        return AMF_FAIL;
      void *native = surface->GetPlaneAt(0)->GetNative();
      if (!native)
        return AMF_FAIL;
      int32_t key_obj = 0;
      auto start = util::now();
      res = encode(native, util_encode::vram_encode_test_callback, &key_obj, 0);
      int64_t elapsed = util::elapsed_ms(start);
      if (res == AMF_OK && key_obj == 1 && elapsed < TEST_TIMEOUT_MS) {
        return AMF_OK;
      }
      return AMF_FAIL;
    } catch (...) {
      return AMF_FAIL;
    }
  }

  AMF_RESULT initialize() {
    AMF_RESULT res;

    res = AMFFactory_.Init();
    if (res != AMF_OK) {
      std::cerr << "AMF init failed, error code = " << res << "\n";
      return res;
    }
    amf::AMFSetCustomTracer(AMFFactory_.GetTrace());
    amf::AMFTraceEnableWriter(AMF_TRACE_WRITER_CONSOLE, true);
    amf::AMFTraceSetWriterLevel(AMF_TRACE_WRITER_CONSOLE, AMF_TRACE_WARNING);

    // AMFContext_
    res = AMFFactory_.GetFactory()->CreateContext(&AMFContext_);
    AMF_CHECK_RETURN(res, "CreateContext failed");

    switch (AMFMemoryType_) {
    case amf::AMF_MEMORY_DX11:
      // NOTE: Windows-specific AMF initialization using DirectX 11
      // The handle_ is an ID3D11Device* obtained from Rust FFI (hwcodec_adapters_get_adapter_device)
      // This is the only Windows platform-specific code in this file - it's required by AMF SDK
      res = AMFContext_->InitDX11(handle_); // can be DX11 device
      AMF_CHECK_RETURN(res, "InitDX11 failed");
      break;
    default:
      LOG_ERROR(std::string("unsupported amf memory type"));
      return AMF_FAIL;
    }

    // component: encoder
    res = AMFFactory_.GetFactory()->CreateComponent(AMFContext_, codec_.c_str(),
                                                    &AMFEncoder_);
    AMF_CHECK_RETURN(res, "CreateComponent failed");

    res = SetParams(codec_);
    AMF_CHECK_RETURN(res, "Could not set params in encoder.");

    // HEVC encoder requires NV12 format, not BGRA
    amf::AMF_SURFACE_FORMAT initFormat = AMFSurfaceFormat_;
    if (codec_ == amf_wstring(AMFVideoEncoder_HEVC)) {
      initFormat = amf::AMF_SURFACE_NV12;
    }

    res = AMFEncoder_->Init(initFormat, resolution_.first,
                            resolution_.second);
    AMF_CHECK_RETURN(res, "encoder->Init() failed");

    return AMF_OK;
  }

private:
  AMF_RESULT SetParams(const amf_wstring &codecStr) {
    AMF_RESULT res;
    if (codecStr == amf_wstring(AMFVideoEncoderVCE_AVC)) {
      // ------------- Encoder params usage---------------
      // Use LOW_LATENCY instead of LOW_LATENCY_HIGH_QUALITY to avoid assertion failures
      // on drivers that don't support usage value 5
      res = AMFEncoder_->SetProperty(
          AMF_VIDEO_ENCODER_USAGE,
          AMF_VIDEO_ENCODER_USAGE_LOW_LATENCY);
      // Don't fail if usage setting fails - some drivers may not support all usage values

      // ------------- Encoder params static---------------
      res = AMFEncoder_->SetProperty(
          AMF_VIDEO_ENCODER_FRAMESIZE,
          ::AMFConstructSize(resolution_.first, resolution_.second));
      AMF_CHECK_RETURN(res,
                       "SetProperty AMF_VIDEO_ENCODER_FRAMESIZE failed, (" +
                           std::to_string(resolution_.first) + "," +
                           std::to_string(resolution_.second) + ")");
      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_LOWLATENCY_MODE, true);
      AMF_CHECK_RETURN(res,
                       "SetProperty AMF_VIDEO_ENCODER_LOWLATENCY_MODE failed");
      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_QUALITY_PRESET,
                                     AMF_VIDEO_ENCODER_QUALITY_PRESET_QUALITY);
      AMF_CHECK_RETURN(res,
                       "SetProperty AMF_VIDEO_ENCODER_QUALITY_PRESET failed");
      // COLOR_BIT_DEPTH may not be supported on all drivers - make it optional
      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_COLOR_BIT_DEPTH, eDepth_);
      if (res != AMF_OK) {
        // Log warning but don't fail - this property is optional
        LOG_DEBUG(std::string("SetProperty AMF_VIDEO_ENCODER_COLOR_BIT_DEPTH failed (not supported), continuing"));
      }
      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD,
                                     AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CBR);
      AMF_CHECK_RETURN(res,
                       "SetProperty AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD");
      if (enable4K_) {
        res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_PROFILE,
                                       AMF_VIDEO_ENCODER_PROFILE_HIGH);
        AMF_CHECK_RETURN(res, "SetProperty(AMF_VIDEO_ENCODER_PROFILE failed");

        res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_PROFILE_LEVEL,
                                       AMF_H264_LEVEL__5_1);
        AMF_CHECK_RETURN(res,
                         "SetProperty AMF_VIDEO_ENCODER_PROFILE_LEVEL failed");
      }
      // color
      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_FULL_RANGE_COLOR,
                                     full_range_);
      AMF_CHECK_RETURN(res, "SetProperty AMF_VIDEO_ENCODER_FULL_RANGE_COLOR");
      // OUTPUT_COLOR_PROFILE may not be supported on all drivers - make it optional
      res = AMFEncoder_->SetProperty<amf_int64>(
          AMF_VIDEO_ENCODER_OUTPUT_COLOR_PROFILE,
          bt709_ ? (full_range_ ? AMF_VIDEO_CONVERTER_COLOR_PROFILE_FULL_709
                                : AMF_VIDEO_CONVERTER_COLOR_PROFILE_709)
                 : (full_range_ ? AMF_VIDEO_CONVERTER_COLOR_PROFILE_FULL_601
                                : AMF_VIDEO_CONVERTER_COLOR_PROFILE_601));
      if (res != AMF_OK) {
        // Log warning but don't fail - this property is optional
        LOG_DEBUG(std::string("SetProperty AMF_VIDEO_ENCODER_OUTPUT_COLOR_PROFILE failed (not supported), continuing"));
      }
      // https://github.com/obsproject/obs-studio/blob/e27b013d4754e0e81119ab237ffedce8fcebcbbf/plugins/obs-ffmpeg/texture-amf.cpp#L924
      // OUTPUT_TRANSFER_CHARACTERISTIC may not be supported on all drivers - make it optional
      res = AMFEncoder_->SetProperty<amf_int64>(
          AMF_VIDEO_ENCODER_OUTPUT_TRANSFER_CHARACTERISTIC,
          bt709_ ? AMF_COLOR_TRANSFER_CHARACTERISTIC_BT709
                 : AMF_COLOR_TRANSFER_CHARACTERISTIC_SMPTE170M);
      if (res != AMF_OK) {
        LOG_DEBUG(std::string("SetProperty AMF_VIDEO_ENCODER_OUTPUT_TRANSFER_CHARACTERISTIC failed (not supported), continuing"));
      }
      // OUTPUT_COLOR_PRIMARIES may not be supported on all drivers - make it optional
      res = AMFEncoder_->SetProperty<amf_int64>(
          AMF_VIDEO_ENCODER_OUTPUT_COLOR_PRIMARIES,
          bt709_ ? AMF_COLOR_PRIMARIES_BT709 : AMF_COLOR_PRIMARIES_SMPTE170M);
      if (res != AMF_OK) {
        LOG_DEBUG(std::string("SetProperty AMF_VIDEO_ENCODER_OUTPUT_COLOR_PRIMARIES failed (not supported), continuing"));
      }

      // ------------- Encoder params dynamic ---------------
      AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_B_PIC_PATTERN, 0);
      // do not check error for AMF_VIDEO_ENCODER_B_PIC_PATTERN
      // - can be not supported - check Capability Manager
      // sample
      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_QUERY_TIMEOUT,
                                     query_timeout_); // ms
      AMF_CHECK_RETURN(res,
                       "SetProperty AMF_VIDEO_ENCODER_QUERY_TIMEOUT failed");
      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE,
                                     bitRateIn_);
      AMF_CHECK_RETURN(res,
                       "SetProperty AMF_VIDEO_ENCODER_TARGET_BITRATE failed");
      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE,
                                     ::AMFConstructRate(frameRate_, 1));
      AMF_CHECK_RETURN(res, "SetProperty AMF_VIDEO_ENCODER_FRAMERATE failed");

      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_IDR_PERIOD, gop_);
      AMF_CHECK_RETURN(res, "SetProperty AMF_VIDEO_ENCODER_IDR_PERIOD failed");

    } else if (codecStr == amf_wstring(AMFVideoEncoder_HEVC)) {
      // ------------- Encoder params usage---------------
      // Use LOW_LATENCY instead of LOW_LATENCY_HIGH_QUALITY to avoid assertion failures
      // on drivers that don't support usage value 5
      res = AMFEncoder_->SetProperty(
          AMF_VIDEO_ENCODER_HEVC_USAGE,
          AMF_VIDEO_ENCODER_HEVC_USAGE_LOW_LATENCY);
      // Don't fail if usage setting fails - some drivers may not support all usage values

      // ------------- Encoder params static---------------
      res = AMFEncoder_->SetProperty(
          AMF_VIDEO_ENCODER_HEVC_FRAMESIZE,
          ::AMFConstructSize(resolution_.first, resolution_.second));
      AMF_CHECK_RETURN(res,
                       "SetProperty AMF_VIDEO_ENCODER_HEVC_FRAMESIZE failed");

      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_HEVC_LOWLATENCY_MODE,
                                     true);
      AMF_CHECK_RETURN(res,
                       "SetProperty AMF_VIDEO_ENCODER_LOWLATENCY_MODE failed");

      res = AMFEncoder_->SetProperty(
          AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET,
          AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET_QUALITY);
      AMF_CHECK_RETURN(
          res, "SetProperty AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET failed");

      // COLOR_BIT_DEPTH may not be supported on all drivers - make it optional
      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH, eDepth_);
      if (res != AMF_OK) {
        // Log warning but don't fail - this property is optional
        LOG_DEBUG(std::string("SetProperty AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH failed (not supported), continuing"));
      }

      res = AMFEncoder_->SetProperty(
          AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD,
          AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_CBR);
      AMF_CHECK_RETURN(
          res, "SetProperty AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD failed");

      if (enable4K_) {
        res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_HEVC_TIER,
                                       AMF_VIDEO_ENCODER_HEVC_TIER_HIGH);
        AMF_CHECK_RETURN(res, "SetProperty(AMF_VIDEO_ENCODER_HEVC_TIER failed");

        res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL,
                                       AMF_LEVEL_5_1);
        AMF_CHECK_RETURN(
            res, "SetProperty AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL failed");
      }
      // color
      // NOMINAL_RANGE may not be supported on all drivers - make it optional
      res = AMFEncoder_->SetProperty<amf_int64>(
          AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE,
          full_range_ ? AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE_FULL
                      : AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE_STUDIO);
      if (res != AMF_OK) {
        // Log warning but don't fail - this property is optional
        LOG_DEBUG(std::string("SetProperty AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE failed (not supported), continuing"));
      }
      // OUTPUT_COLOR_PROFILE may not be supported on all drivers - make it optional
      res = AMFEncoder_->SetProperty<amf_int64>(
          AMF_VIDEO_ENCODER_HEVC_OUTPUT_COLOR_PROFILE,
          bt709_ ? (full_range_ ? AMF_VIDEO_CONVERTER_COLOR_PROFILE_FULL_709
                                : AMF_VIDEO_CONVERTER_COLOR_PROFILE_709)
                 : (full_range_ ? AMF_VIDEO_CONVERTER_COLOR_PROFILE_FULL_601
                                : AMF_VIDEO_CONVERTER_COLOR_PROFILE_601));
      if (res != AMF_OK) {
        // Log warning but don't fail - this property is optional
        LOG_DEBUG(std::string("SetProperty AMF_VIDEO_ENCODER_HEVC_OUTPUT_COLOR_PROFILE failed (not supported), continuing"));
      }
      // OUTPUT_TRANSFER_CHARACTERISTIC may not be supported on all drivers - make it optional
      res = AMFEncoder_->SetProperty<amf_int64>(
          AMF_VIDEO_ENCODER_HEVC_OUTPUT_TRANSFER_CHARACTERISTIC,
          bt709_ ? AMF_COLOR_TRANSFER_CHARACTERISTIC_BT709
                 : AMF_COLOR_TRANSFER_CHARACTERISTIC_SMPTE170M);
      if (res != AMF_OK) {
        LOG_DEBUG(std::string("SetProperty AMF_VIDEO_ENCODER_HEVC_OUTPUT_TRANSFER_CHARACTERISTIC failed (not supported), continuing"));
      }
      // OUTPUT_COLOR_PRIMARIES may not be supported on all drivers - make it optional
      res = AMFEncoder_->SetProperty<amf_int64>(
          AMF_VIDEO_ENCODER_HEVC_OUTPUT_COLOR_PRIMARIES,
          bt709_ ? AMF_COLOR_PRIMARIES_BT709 : AMF_COLOR_PRIMARIES_SMPTE170M);
      if (res != AMF_OK) {
        LOG_DEBUG(std::string("SetProperty AMF_VIDEO_ENCODER_HEVC_OUTPUT_COLOR_PRIMARIES failed (not supported), continuing"));
      }

      // ------------- Encoder params dynamic ---------------
      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_HEVC_QUERY_TIMEOUT,
                                     query_timeout_); // ms
      AMF_CHECK_RETURN(
          res, "SetProperty(AMF_VIDEO_ENCODER_HEVC_QUERY_TIMEOUT failed");

      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE,
                                     bitRateIn_);
      AMF_CHECK_RETURN(
          res, "SetProperty AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE failed");

      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE,
                                     ::AMFConstructRate(frameRate_, 1));
      AMF_CHECK_RETURN(res,
                       "SetProperty AMF_VIDEO_ENCODER_HEVC_FRAMERATE failed");

      res = AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_HEVC_GOP_SIZE,
                                     gop_); // todo
      AMF_CHECK_RETURN(res,
                       "SetProperty AMF_VIDEO_ENCODER_HEVC_GOP_SIZE failed");
    } else {
      return AMF_FAIL;
    }
    return AMF_OK;
  }

  void PacketKeyframe(amf::AMFDataPtr &pData, struct encoder_packet *packet) {
    if (AMFVideoEncoderVCE_AVC == codec_) {
      uint64_t pktType;
      pData->GetProperty(AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE, &pktType);
      packet->keyframe = AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_IDR == pktType ||
                         AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_I == pktType;
    } else if (AMFVideoEncoder_HEVC == codec_) {
      uint64_t pktType;
      pData->GetProperty(AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE, &pktType);
      packet->keyframe =
          AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_IDR == pktType ||
          AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_I == pktType;
    }
  }
};

bool convert_codec(DataFormat lhs, amf_wstring &rhs) {
  switch (lhs) {
  case H264:
    rhs = AMFVideoEncoderVCE_AVC;
    break;
  case H265:
    rhs = AMFVideoEncoder_HEVC;
    break;
  default:
    LOG_ERROR(std::string("unsupported codec: ") + std::to_string((int)lhs));
    return false;
  }
  return true;
}

} // namespace
#include "amf_common.cpp"

extern "C" {

int amf_destroy_encoder(void *encoder) {
  try {
    AMFEncoder *enc = (AMFEncoder *)encoder;
    enc->destroy();
    delete enc;
    enc = NULL;
    return 0;
  } catch (const std::exception &e) {
          LOG_ERROR(std::string("destroy failed: ") + e.what());
  }
  return -1;
}

void *amf_new_encoder(void *handle, int64_t luid,
                      DataFormat dataFormat, int32_t width, int32_t height,
                      int32_t kbs, int32_t framerate, int32_t gop) {
  (void)luid;
  AMFEncoder *enc = NULL;
  try {
    amf_wstring codecStr;
    if (!convert_codec(dataFormat, codecStr)) {
      return NULL;
    }
    amf::AMF_MEMORY_TYPE memoryType;
    if (!convert_api(memoryType)) {
      return NULL;
    }
    enc = new AMFEncoder(handle, memoryType, codecStr, dataFormat, width,
                         height, kbs * 1000, framerate, gop);
    if (enc) {
      if (AMF_OK == enc->initialize()) {
        return enc;
      }
    }
  } catch (const std::exception &e) {
          LOG_ERROR(std::string("new failed: ") + e.what());
  }
  if (enc) {
    enc->destroy();
    delete enc;
    enc = NULL;
  }
  return NULL;
}

int amf_encode(void *encoder, void *tex, EncodeCallback callback, void *obj,
               int64_t ms) {
  try {
    AMFEncoder *enc = (AMFEncoder *)encoder;
    return -enc->encode(tex, callback, obj, ms);
  } catch (const std::exception &e) {
          LOG_ERROR(std::string("encode failed: ") + e.what());
  }
  return -1;
}

int amf_driver_support() {
  try {
    AMFFactoryHelper factory;
    AMF_RESULT res = factory.Init();
    if (res == AMF_OK) {
      factory.Terminate();
      return 0;
    }
  } catch (const std::exception &) {
  }
  return -1;
}

int amf_test_encode(int64_t *outLuids, int32_t *outVendors, int32_t maxDescNum, int32_t *outDescNum,
                    DataFormat dataFormat, int32_t width,
                    int32_t height, int32_t kbs, int32_t framerate,
                    int32_t gop, const int64_t *excludedLuids, const int32_t *excludeFormats, int32_t excludeCount) {
  try {
    AdaptersHandle adapters = hwcodec_adapters_new(AdapterVendor::ADAPTER_VENDOR_AMD);
    if (!adapters)
      return -1;
    int count = 0;
    int adapter_count = hwcodec_adapters_get_count(adapters);
    for (int i = 0; i < adapter_count; i++) {
      int64_t currentLuid = hwcodec_adapters_get_adapter_luid(adapters, i);
      if (util::skip_test(excludedLuids, excludeFormats, excludeCount, currentLuid, dataFormat)) {
        continue;
      }
      
      AMFEncoder *e = nullptr;
      try {
        ID3D11Device* device = (ID3D11Device*)hwcodec_adapters_get_adapter_device(adapters, i);
        e = (AMFEncoder *)amf_new_encoder(
            (void *)device, currentLuid,
            dataFormat, width, height, kbs, framerate, gop);
        if (!e)
          continue;
        
        AMF_RESULT test_res = e->test();
        if (test_res == AMF_OK) {
          outLuids[count] = currentLuid;
          outVendors[count] = Vendor::VENDOR_AMD;
          count += 1;
        }
      } catch (const std::exception &ex) {
        LOG_ERROR(std::string("AMF encoder test exception: ") + ex.what());
      } catch (...) {
        LOG_ERROR(std::string("AMF encoder test unknown exception"));
      }
      
      if (e) {
        try {
          e->destroy();
          delete e;
        } catch (...) {
          // Ignore cleanup errors
        }
        e = nullptr;
      }
      
      if (count >= maxDescNum)
        break;
    }
    hwcodec_adapters_destroy(adapters);
    *outDescNum = count;
    return 0;

  } catch (const std::exception &e) {
    LOG_ERROR(std::string("test ") + std::to_string(kbs) + " failed: " + e.what());
  } catch (...) {
    LOG_ERROR(std::string("test unknown exception"));
  }
  return -1;
}

int amf_set_bitrate(void *encoder, int32_t kbs) {
  try {
    AMFEncoder *enc = (AMFEncoder *)encoder;
    AMF_RESULT res = AMF_FAIL;
    switch (enc->dataFormat_) {
    case H264:
      res = enc->AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_TARGET_BITRATE,
                                          kbs * 1000);
      break;
    case H265:
      res = enc->AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE,
                                          kbs * 1000);
      break;
    }
    return res == AMF_OK ? 0 : -1;
  } catch (const std::exception &e) {
    LOG_ERROR(std::string("set bitrate to ") + std::to_string(kbs) +
              "k failed: " + e.what());
  }
  return -1;
}

int amf_set_framerate(void *encoder, int32_t framerate) {
  try {
    AMFEncoder *enc = (AMFEncoder *)encoder;
    AMF_RESULT res = AMF_FAIL;
    AMFRate rate = ::AMFConstructRate(framerate, 1);
    switch (enc->dataFormat_) {
    case H264:
      res = enc->AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_FRAMERATE, rate);
      break;
    case H265:
      res =
          enc->AMFEncoder_->SetProperty(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, rate);
      break;
    }
    return res == AMF_OK ? 0 : -1;
  } catch (const std::exception &e) {
    LOG_ERROR(std::string("set framerate to ") + std::to_string(framerate) +
              " failed: " + e.what());
  }
  return -1;
}

} // extern "C"