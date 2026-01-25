#ifndef SYSTEM_H
#define SYSTEM_H

#ifdef _WIN32
// ============================================================================
// Windows 平台实现已完全替换为 Rust
// ============================================================================
// 
// 所有 Windows 平台代码（win.cpp, bmp.cpp, dump.cpp）已替换为 Rust 实现
// 通过 FFI 接口导出（win_rust_ffi.h）
//
// 使用 Rust FFI 接口头文件（主要接口）
#include "platform/win/win_rust_ffi.h"
// 
// 保留 win.h 仅用于：
//   - LUID 宏定义（某些 C++ 代码仍在使用）
//   - 向后兼容
// 注意：win.h 中的类定义已不再使用，实现都在 Rust 中
#include "platform/win/win.h"
#endif
#ifdef __linux__
#include "platform/linux/linux.h"
#endif

#endif // SYSTEM_H