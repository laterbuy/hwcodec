#pragma once

// NVIDIA Video Codec SDK bridge header - 只包含类型声明，无业务逻辑
// 用于 cxx crate 桥接 NVIDIA SDK 的 C++ 类型到 Rust

// 注意：NVIDIA SDK 主要是 C++ 类，需要通过 cxx 暴露类型
// 所有业务逻辑将在 Rust 中实现
