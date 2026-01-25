#pragma once

// AMF SDK bridge header - 只包含类型声明，无业务逻辑
// 用于 cxx crate 桥接 AMF SDK 的 C++ 类型到 Rust

#include <public/common/AMFFactory.h>
#include <public/include/core/Factory.h>
#include <public/include/core/Context.h>
#include <public/include/components/Component.h>
#include <public/include/core/Data.h>
#include <public/include/core/Variant.h>

// 只暴露必要的类型，不包含任何业务逻辑
// 所有业务逻辑将在 Rust 中实现
