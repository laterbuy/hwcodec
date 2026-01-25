#include "common.h"
#include <iostream>
#include <public/common/TraceAdapter.h>
#include <stdio.h>

#ifndef AMF_FACILITY
#define AMF_FACILITY L"AMFCommon"
#endif

static bool convert_api(amf::AMF_MEMORY_TYPE &rhs) {
  // NOTE: Windows platform-specific - AMF on Windows only supports DirectX 11
  // This is required by AMF SDK and cannot be replaced with Rust
  // The actual D3D11 device management is handled by Rust (via FFI)
  rhs = amf::AMF_MEMORY_DX11;
  return true;
}

static bool convert_surface_format(SurfaceFormat lhs,
                                   amf::AMF_SURFACE_FORMAT &rhs) {
  switch (lhs) {
  case SURFACE_FORMAT_NV12:
    rhs = amf::AMF_SURFACE_NV12;
    break;
  case SURFACE_FORMAT_RGBA:
    rhs = amf::AMF_SURFACE_RGBA;
    break;
  case SURFACE_FORMAT_BGRA:
    rhs = amf::AMF_SURFACE_BGRA;
    break;
  default:
    std::cerr << "unsupported surface format: " << static_cast<int>(lhs)
              << "\n";
    return false;
  }
  return true;
}
