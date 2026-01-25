// AMF SDK C 包装层实现
// 只包装 SDK 的 C++ 接口调用，不包含业务逻辑
// 所有业务逻辑在 Rust 中实现

#include "amf_wrapper.h"
#include "../common/common.h"
#include <public/common/AMFFactory.h>
#include <public/common/AMFSTL.h>
#include <public/include/core/Factory.h>
#include <public/include/core/Context.h>
#include <public/include/components/Component.h>
#include <public/include/core/Data.h>
#include <public/include/core/Variant.h>
#include <public/include/core/Surface.h>
#include <public/include/core/Plane.h>
#include <public/include/core/Buffer.h>
#include <public/include/core/Platform.h>
#include <public/include/core/Result.h>
#include <public/include/components/VideoConverter.h>
#include <d3d11.h>
#include <cstring>
#include <string>
#include <cstdint>

#ifdef __cplusplus
extern "C" {
#endif

// AMF SDK basic functions
int amf_wrapper_factory_init(void** factory) {
    try {
        AMFFactoryHelper* helper = new AMFFactoryHelper();
        AMF_RESULT res = helper->Init();
        if (res != AMF_OK) {
            delete helper;
            return -1;
        }
        *factory = helper;
        return 0;
    } catch (...) {
        return -1;
    }
}

void amf_wrapper_factory_terminate(void* factory) {
    if (factory) {
        AMFFactoryHelper* helper = static_cast<AMFFactoryHelper*>(factory);
        helper->Terminate();
        delete helper;
    }
}

int amf_wrapper_create_context(void* factory, void** context) {
    try {
        AMFFactoryHelper* helper = static_cast<AMFFactoryHelper*>(factory);
        amf::AMFContextPtr ctx;
        AMF_RESULT res = helper->GetFactory()->CreateContext(&ctx);
        if (res != AMF_OK) {
            return -1;
        }
        *context = ctx.Detach();
        return 0;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_context_init_dx11(void* context, void* device) {
    try {
        amf::AMFContext* ctx = static_cast<amf::AMFContext*>(context);
        AMF_RESULT res = ctx->InitDX11(static_cast<ID3D11Device*>(device));
        return (res == AMF_OK) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_create_encoder_component(void* factory, void* context, const char* codec_name, void** component) {
    try {
        AMFFactoryHelper* helper = static_cast<AMFFactoryHelper*>(factory);
        amf::AMFContext* ctx = static_cast<amf::AMFContext*>(context);
        
        // Convert char* to amf_wstring (wchar_t*)
        amf_wstring codec_wstr;
        size_t len = strlen(codec_name);
        codec_wstr.resize(len);
        for (size_t i = 0; i < len; i++) {
            codec_wstr[i] = static_cast<wchar_t>(codec_name[i]);
        }
        
        amf::AMFComponentPtr comp;
        AMF_RESULT res = helper->GetFactory()->CreateComponent(ctx, codec_wstr.c_str(), &comp);
        if (res != AMF_OK) {
            return -1;
        }
        *component = comp.Detach();
        return 0;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_create_decoder_component(void* factory, void* context, const char* codec_name, void** component) {
    try {
        AMFFactoryHelper* helper = static_cast<AMFFactoryHelper*>(factory);
        amf::AMFContext* ctx = static_cast<amf::AMFContext*>(context);
        
        // Convert char* to amf_wstring (wchar_t*)
        amf_wstring codec_wstr;
        size_t len = strlen(codec_name);
        codec_wstr.resize(len);
        for (size_t i = 0; i < len; i++) {
            codec_wstr[i] = static_cast<wchar_t>(codec_name[i]);
        }
        
        amf::AMFComponentPtr comp;
        AMF_RESULT res = helper->GetFactory()->CreateComponent(ctx, codec_wstr.c_str(), &comp);
        if (res != AMF_OK) {
            return -1;
        }
        *component = comp.Detach();
        return 0;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_create_converter_component(void* factory, void* context, void** component) {
    try {
        AMFFactoryHelper* helper = static_cast<AMFFactoryHelper*>(factory);
        amf::AMFContext* ctx = static_cast<amf::AMFContext*>(context);
        
        amf::AMFComponentPtr comp;
        AMF_RESULT res = helper->GetFactory()->CreateComponent(ctx, AMFVideoConverter, &comp);
        if (res != AMF_OK) {
            return -1;
        }
        *component = comp.Detach();
        return 0;
    } catch (...) {
        return -1;
    }
}

// Component operations
int amf_wrapper_component_set_property_int64(void* component, const char* name, int64_t value) {
    try {
        amf::AMFComponent* comp = static_cast<amf::AMFComponent*>(component);
        amf_wstring name_wstr;
        size_t len = strlen(name);
        name_wstr.resize(len);
        for (size_t i = 0; i < len; i++) {
            name_wstr[i] = static_cast<wchar_t>(name[i]);
        }
        AMF_RESULT res = comp->SetProperty(name_wstr.c_str(), value);
        return (res == AMF_OK) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_component_set_property_int32(void* component, const char* name, int32_t value) {
    try {
        amf::AMFComponent* comp = static_cast<amf::AMFComponent*>(component);
        amf_wstring name_wstr;
        size_t len = strlen(name);
        name_wstr.resize(len);
        for (size_t i = 0; i < len; i++) {
            name_wstr[i] = static_cast<wchar_t>(name[i]);
        }
        AMF_RESULT res = comp->SetProperty(name_wstr.c_str(), value);
        return (res == AMF_OK) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_component_set_property_bool(void* component, const char* name, int32_t value) {
    try {
        amf::AMFComponent* comp = static_cast<amf::AMFComponent*>(component);
        amf_wstring name_wstr;
        size_t len = strlen(name);
        name_wstr.resize(len);
        for (size_t i = 0; i < len; i++) {
            name_wstr[i] = static_cast<wchar_t>(name[i]);
        }
        AMF_RESULT res = comp->SetProperty(name_wstr.c_str(), value != 0);
        return (res == AMF_OK) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_component_set_property_double(void* component, const char* name, double value) {
    try {
        amf::AMFComponent* comp = static_cast<amf::AMFComponent*>(component);
        amf_wstring name_wstr;
        size_t len = strlen(name);
        name_wstr.resize(len);
        for (size_t i = 0; i < len; i++) {
            name_wstr[i] = static_cast<wchar_t>(name[i]);
        }
        AMF_RESULT res = comp->SetProperty(name_wstr.c_str(), value);
        return (res == AMF_OK) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_component_set_property_wstring(void* component, const char* name, const wchar_t* value) {
    try {
        amf::AMFComponent* comp = static_cast<amf::AMFComponent*>(component);
        amf_wstring name_wstr;
        size_t len = strlen(name);
        name_wstr.resize(len);
        for (size_t i = 0; i < len; i++) {
            name_wstr[i] = static_cast<wchar_t>(name[i]);
        }
        amf_wstring value_wstr(value);
        AMF_RESULT res = comp->SetProperty(name_wstr.c_str(), value_wstr.c_str());
        return (res == AMF_OK) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_component_init(void* component, int32_t format, int32_t width, int32_t height) {
    try {
        amf::AMFComponent* comp = static_cast<amf::AMFComponent*>(component);
        AMF_RESULT res = comp->Init(static_cast<amf::AMF_SURFACE_FORMAT>(format), width, height);
        return (res == AMF_OK) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

void amf_wrapper_component_terminate(void* component) {
    if (component) {
        try {
            amf::AMFComponent* comp = static_cast<amf::AMFComponent*>(component);
            comp->Terminate();
        } catch (...) {
        }
    }
}

int amf_wrapper_component_drain(void* component) {
    if (!component) {
        return -1;
    }
    try {
        amf::AMFComponent* comp = static_cast<amf::AMFComponent*>(component);
        AMF_RESULT res = comp->Drain();
        return (res == AMF_OK) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

// Surface operations
int amf_wrapper_create_surface_from_dx11(void* context, void* texture, void** surface) {
    try {
        amf::AMFContext* ctx = static_cast<amf::AMFContext*>(context);
        amf::AMFSurfacePtr surf;
        AMF_RESULT res = ctx->CreateSurfaceFromDX11Native(static_cast<ID3D11Texture2D*>(texture), &surf, nullptr);
        if (res != AMF_OK) {
            return -1;
        }
        *surface = surf.Detach();
        return 0;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_alloc_surface(void* context, int32_t memory_type, int32_t format, int32_t width, int32_t height, void** surface) {
    try {
        amf::AMFContext* ctx = static_cast<amf::AMFContext*>(context);
        amf::AMFSurfacePtr surf;
        AMF_RESULT res = ctx->AllocSurface(
            static_cast<amf::AMF_MEMORY_TYPE>(memory_type),
            static_cast<amf::AMF_SURFACE_FORMAT>(format),
            width, height, &surf);
        if (res != AMF_OK) {
            return -1;
        }
        *surface = surf.Detach();
        return 0;
    } catch (...) {
        return -1;
    }
}

void amf_wrapper_surface_set_pts(void* surface, int64_t pts) {
    if (surface) {
        try {
            amf::AMFSurface* surf = static_cast<amf::AMFSurface*>(surface);
            // AMF_MILLISECOND is defined in Platform.h as (AMF_SECOND / 1000)
            // AMF_SECOND is 10000000 (100ns units)
            // AMF_MILLISECOND = 10000
            amf_pts amf_pts_value = pts * 10000;
            surf->SetPts(amf_pts_value);
        } catch (...) {
        }
    }
}

int amf_wrapper_surface_duplicate(void* surface, int32_t memory_type, void** new_surface) {
    try {
        amf::AMFSurface* surf = static_cast<amf::AMFSurface*>(surface);
        amf::AMFDataPtr data;
        AMF_RESULT res = surf->Duplicate(static_cast<amf::AMF_MEMORY_TYPE>(memory_type), &data);
        if (res != AMF_OK) {
            return -1;
        }
        amf::AMFSurfacePtr new_surf(data);
        amf::AMFSurface* new_surf_ptr = new_surf.Detach();
        *new_surface = new_surf_ptr;
        return 0;
    } catch (...) {
        return -1;
    }
}

// Encoder operations
int amf_wrapper_encoder_submit_input(void* encoder, void* surface) {
    try {
        amf::AMFComponent* enc = static_cast<amf::AMFComponent*>(encoder);
        amf::AMFSurface* surf = static_cast<amf::AMFSurface*>(surface);
        AMF_RESULT res = enc->SubmitInput(surf);
        return (res == AMF_OK) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_encoder_query_output(void* encoder, void** data) {
    try {
        amf::AMFComponent* enc = static_cast<amf::AMFComponent*>(encoder);
        amf::AMFDataPtr output;
        AMF_RESULT res = enc->QueryOutput(&output);
        if (res == AMF_OK && output != nullptr) {
            *data = output.Detach();
            return 0;
        } else if (res == AMF_REPEAT) {
            return 1; // Need more input
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

// Decoder operations
int amf_wrapper_decoder_submit_input(void* decoder, const uint8_t* data, int32_t size, int64_t pts) {
    try {
        amf::AMFComponent* dec = static_cast<amf::AMFComponent*>(decoder);
        amf::AMFBufferPtr buffer;
        AMF_RESULT res = dec->GetContext()->AllocBuffer(amf::AMF_MEMORY_HOST, size, &buffer);
        if (res != AMF_OK) {
            return -1;
        }
        memcpy(buffer->GetNative(), data, size);
        // AMF_MILLISECOND = 10000 (AMF_SECOND / 1000, where AMF_SECOND = 10000000)
        amf_pts amf_pts_value = pts * 10000;
        buffer->SetPts(amf_pts_value);
        res = dec->SubmitInput(buffer);
        if (res == AMF_OK) {
            return 0;
        } else if (res == AMF_RESOLUTION_CHANGED) {
            return 2; // 分辨率变化
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_decoder_query_output(void* decoder, void** surface) {
    try {
        amf::AMFComponent* dec = static_cast<amf::AMFComponent*>(decoder);
        amf::AMFDataPtr output;
        AMF_RESULT res = dec->QueryOutput(&output);
        if (res == AMF_OK && output != nullptr) {
            *surface = output.Detach();
            return 0;
        } else if (res == AMF_REPEAT) {
            return 1; // Need more input
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

// Converter operations
int amf_wrapper_converter_submit_input(void* converter, void* surface) {
    try {
        amf::AMFComponent* conv = static_cast<amf::AMFComponent*>(converter);
        amf::AMFSurface* surf = static_cast<amf::AMFSurface*>(surface);
        AMF_RESULT res = conv->SubmitInput(surf);
        return (res == AMF_OK) ? 0 : -1;
    } catch (...) {
        return -1;
    }
}

int amf_wrapper_converter_query_output(void* converter, void** data) {
    try {
        amf::AMFComponent* conv = static_cast<amf::AMFComponent*>(converter);
        amf::AMFDataPtr output;
        AMF_RESULT res = conv->QueryOutput(&output);
        if (res == AMF_OK && output != nullptr) {
            *data = output.Detach();
            return 0;
        } else if (res == AMF_REPEAT) {
            return 1; // Need more input
        }
        return -1;
    } catch (...) {
        return -1;
    }
}

// Buffer operations
int32_t amf_wrapper_buffer_get_size(void* buffer) {
    if (buffer) {
        try {
            amf::AMFBuffer* buf = static_cast<amf::AMFBuffer*>(buffer);
            return static_cast<int32_t>(buf->GetSize());
        } catch (...) {
        }
    }
    return 0;
}

void* amf_wrapper_buffer_get_native(void* buffer) {
    if (buffer) {
        try {
            amf::AMFBuffer* buf = static_cast<amf::AMFBuffer*>(buffer);
            return buf->GetNative();
        } catch (...) {
        }
    }
    return nullptr;
}

int amf_wrapper_buffer_get_property_int64(void* buffer, const char* name, int64_t* value) {
    if (!buffer || !name || !value) {
        return -1;
    }
    try {
        amf::AMFBuffer* buf = static_cast<amf::AMFBuffer*>(buffer);
        // Convert char* to amf_wstring (wchar_t*)
        amf_wstring propName;
        size_t len = strlen(name);
        propName.resize(len);
        for (size_t i = 0; i < len; i++) {
            propName[i] = static_cast<wchar_t>(name[i]);
        }
        AMF_RESULT res = buf->GetProperty(propName.c_str(), value);
        if (res == AMF_OK) {
            return 0;
        }
    } catch (...) {
    }
    return -1;
}

// Surface operations
int32_t amf_wrapper_surface_get_format(void* surface) {
    if (surface) {
        try {
            amf::AMFSurface* surf = static_cast<amf::AMFSurface*>(surface);
            return static_cast<int32_t>(surf->GetFormat());
        } catch (...) {
        }
    }
    return 0;
}

int32_t amf_wrapper_surface_get_width(void* surface) {
    if (surface) {
        try {
            amf::AMFSurface* surf = static_cast<amf::AMFSurface*>(surface);
            if (surf->GetPlanesCount() > 0) {
                amf::AMFPlane* plane = surf->GetPlaneAt(0);
                if (plane) {
                    return plane->GetWidth();
                }
            }
        } catch (...) {
        }
    }
    return 0;
}

int32_t amf_wrapper_surface_get_height(void* surface) {
    if (surface) {
        try {
            amf::AMFSurface* surf = static_cast<amf::AMFSurface*>(surface);
            if (surf->GetPlanesCount() > 0) {
                amf::AMFPlane* plane = surf->GetPlaneAt(0);
                if (plane) {
                    return plane->GetHeight();
                }
            }
        } catch (...) {
        }
    }
    return 0;
}

int32_t amf_wrapper_surface_get_planes_count(void* surface) {
    if (surface) {
        try {
            amf::AMFSurface* surf = static_cast<amf::AMFSurface*>(surface);
            return static_cast<int32_t>(surf->GetPlanesCount());
        } catch (...) {
        }
    }
    return 0;
}

void* amf_wrapper_surface_get_plane_at(void* surface, int32_t plane_index) {
    if (surface) {
        try {
            amf::AMFSurface* surf = static_cast<amf::AMFSurface*>(surface);
            return surf->GetPlaneAt(plane_index);
        } catch (...) {
        }
    }
    return nullptr;
}

void* amf_wrapper_plane_get_native(void* plane) {
    if (plane) {
        try {
            amf::AMFPlane* pl = static_cast<amf::AMFPlane*>(plane);
            return pl->GetNative();
        } catch (...) {
        }
    }
    return nullptr;
}

// 从主机内存创建 Buffer（用于重新提交输入）
int amf_wrapper_create_buffer_from_host(void* context, const uint8_t* data, int32_t size, void** buffer) {
    try {
        amf::AMFContext* ctx = static_cast<amf::AMFContext*>(context);
        amf::AMFBufferPtr buf;
        AMF_RESULT res = ctx->CreateBufferFromHostNative(const_cast<void*>(reinterpret_cast<const void*>(data)), size, &buf, nullptr);
        if (res != AMF_OK) {
            return -1;
        }
        *buffer = buf.Detach();
        return 0;
    } catch (...) {
        return -1;
    }
}

// Release resources
void amf_wrapper_release(void* ptr) {
    if (ptr) {
        try {
            amf::AMFInterface* iface = static_cast<amf::AMFInterface*>(ptr);
            iface->Release();
        } catch (...) {
        }
    }
}

#ifdef __cplusplus
}
#endif
