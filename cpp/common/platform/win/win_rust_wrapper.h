#ifndef WIN_RUST_WRAPPER_H
#define WIN_RUST_WRAPPER_H

// C++ wrapper classes that use Rust FFI interfaces but provide the same interface as the original NativeDevice class
// This minimizes changes to existing code

#include "win_rust_ffi.h"
#include "../../common.h"
#include <d3d11.h>
#include <d3d11_1.h>
#include <dxgi.h>
#include <wrl/client.h>

using Microsoft::WRL::ComPtr;

// Wrapper class using Rust implementation
class NativeDeviceRust {
public:
  bool Init(int64_t luid, ID3D11Device *device, int pool_size = 1) {
    handle_ = hwcodec_native_device_new(luid, device, pool_size);
    if (!handle_) {
      return false;
    }
    
    // Cache commonly used pointers (ComPtr manages reference counting)
    ID3D11Device* raw_device = hwcodec_native_device_get_device(handle_);
    ID3D11DeviceContext* raw_context = hwcodec_native_device_get_context(handle_);
    ID3D11VideoDevice* raw_video_device = hwcodec_native_device_get_video_device(handle_);
    ID3D11VideoContext* raw_video_context = hwcodec_native_device_get_video_context(handle_);
    ID3D11VideoContext1* raw_video_context1 = hwcodec_native_device_get_video_context1(handle_);
    
    if (raw_device) {
      device_.Attach(raw_device);
    }
    if (raw_context) {
      context_.Attach(raw_context);
    }
    if (raw_video_device) {
      video_device_.Attach(raw_video_device);
    }
    if (raw_video_context) {
      video_context_.Attach(raw_video_context);
    }
    if (raw_video_context1) {
      video_context1_.Attach(raw_video_context1);
    }
    
    // Get adapter (need to query from device)
    if (device_) {
      ComPtr<IDXGIDevice> dxgiDevice;
      HRESULT hr = device_.As(&dxgiDevice);
      if (SUCCEEDED(hr)) {
        dxgiDevice->GetAdapter(&adapter_);
        if (adapter_) {
          adapter_.As(&adapter1_);
        }
      }
    }
    
    return true;
  }

  bool EnsureTexture(int width, int height) {
    return hwcodec_native_device_ensure_texture(handle_, width, height) != 0;
  }

  bool SetTexture(ID3D11Texture2D *texture) {
    hwcodec_native_device_set_texture(handle_, texture);
    return true;
  }

  HANDLE GetSharedHandle() {
    return hwcodec_native_device_get_shared_handle(handle_);
  }

  ID3D11Texture2D *GetCurrentTexture() {
    return hwcodec_native_device_get_current_texture(handle_);
  }

  int next() {
    return hwcodec_native_device_next(handle_);
  }

  void BeginQuery() {
    hwcodec_native_device_begin_query(handle_);
  }

  void EndQuery() {
    hwcodec_native_device_end_query(handle_);
  }

  bool Query() {
    return hwcodec_native_device_query(handle_) != 0;
  }

  bool Process(ID3D11Texture2D *in, ID3D11Texture2D *out, int width, int height,
               D3D11_VIDEO_PROCESSOR_CONTENT_DESC content_desc,
               DXGI_COLOR_SPACE_TYPE colorSpace_in,
               DXGI_COLOR_SPACE_TYPE colorSpace_out, int arraySlice) {
    return hwcodec_native_device_process(handle_, in, out, width, height,
                                        &content_desc, colorSpace_in,
                                        colorSpace_out, arraySlice) != 0;
  }

  bool BgraToNv12(ID3D11Texture2D *bgraTexture, ID3D11Texture2D *nv12Texture,
                  int width, int height, DXGI_COLOR_SPACE_TYPE colorSpace_in,
                  DXGI_COLOR_SPACE_TYPE colorSpace_out) {
    return hwcodec_native_device_bgra_to_nv12(handle_, bgraTexture, nv12Texture,
                                              width, height, colorSpace_in,
                                              colorSpace_out) != 0;
  }

  bool Nv12ToBgra(int width, int height, ID3D11Texture2D *nv12Texture,
                  ID3D11Texture2D *bgraTexture, int nv12ArrayIndex) {
    return hwcodec_native_device_nv12_to_bgra(handle_, nv12Texture, bgraTexture,
                                              width, height, nv12ArrayIndex) != 0;
  }

  AdapterVendor GetVendor() {
    int vendor = hwcodec_native_device_get_vendor(handle_);
    return static_cast<AdapterVendor>(vendor);
  }

  bool support_decode(DataFormat format) {
    int format_int = (format == H264) ? 0 : 1;
    return hwcodec_native_device_support_decode(handle_, format_int) != 0;
  }

  // Public member variables (compatible with original interface)
  ComPtr<IDXGIFactory1> factory1_;
  ComPtr<IDXGIAdapter> adapter_;
  ComPtr<IDXGIAdapter1> adapter1_;
  ComPtr<ID3D11Device> device_;
  ComPtr<ID3D11DeviceContext> context_;
  ComPtr<ID3D11Query> query_;
  ComPtr<ID3D11VideoDevice> video_device_;
  ComPtr<ID3D11VideoContext> video_context_;
  ComPtr<ID3D11VideoContext1> video_context1_;

  ~NativeDeviceRust() {
    if (handle_) {
      hwcodec_native_device_destroy(handle_);
      handle_ = nullptr;
    }
  }

private:
  NativeDeviceHandle handle_;
};

// Wrapper for Adapters class
class AdaptersRust {
public:
  bool Init(AdapterVendor vendor) {
    int vendor_int = static_cast<int>(vendor);
    handle_ = hwcodec_adapters_new(vendor_int);
    if (!handle_) {
      return false;
    }
    count_ = hwcodec_adapters_get_count(handle_);
    return true;
  }

  static int GetFirstAdapterIndex(AdapterVendor vendor) {
    int vendor_int = static_cast<int>(vendor);
    return hwcodec_adapters_get_first_adapter_index(vendor_int);
  }

  int GetCount() const {
    return count_;
  }

  ID3D11Device* GetAdapterDevice(int index) {
    if (index < 0 || index >= count_) {
      return nullptr;
    }
    return hwcodec_adapters_get_adapter_device(handle_, index);
  }

  int64_t GetAdapterLuid(int index) {
    if (index < 0 || index >= count_) {
      return 0;
    }
    return hwcodec_adapters_get_adapter_luid(handle_, index);
  }

  bool GetAdapterDesc(int index, DXGI_ADAPTER_DESC1* desc) {
    if (index < 0 || index >= count_ || !desc) {
      return false;
    }
    return hwcodec_adapters_get_adapter_desc(handle_, index, desc) != 0;
  }

  // Iterator support compatible with original interface
  struct AdapterInfo {
    ID3D11Device* device;
    DXGI_ADAPTER_DESC1 desc;
    int64_t luid;
  };

  AdapterInfo GetAdapterInfo(int index) {
    AdapterInfo info = {};
    if (index >= 0 && index < count_) {
      info.device = GetAdapterDevice(index);
      GetAdapterDesc(index, &info.desc);
      info.luid = GetAdapterLuid(index);
    }
    return info;
  }

  ~AdaptersRust() {
    if (handle_) {
      hwcodec_adapters_destroy(handle_);
      handle_ = nullptr;
    }
  }

private:
  AdaptersHandle handle_;
  int count_;
};

#endif // WIN_RUST_WRAPPER_H
