# 测试指南

## 快速开始

### 运行所有测试
```bash
cargo test
```

### 运行特定测试
```bash
# 运行 GPU 签名测试
cargo test test_gpu_signature

# 运行 FFI 测试
cargo test test_ffi_functions
```

### 运行需要 GPU 的测试（被忽略的测试）
```bash
cargo test -- --ignored
```

### 运行所有测试（包括被忽略的）
```bash
cargo test -- --include-ignored
```

## 测试分类

### 单元测试
位置：`src/platform/win/mod.rs`

这些测试在模块内部，测试单个模块的功能。

运行：
```bash
cargo test --lib
```

### 集成测试
位置：`tests/platform_win_tests.rs`

这些测试测试多个模块的集成。

运行：
```bash
cargo test --test platform_win_tests
```

## 常用测试命令

### 显示测试输出
```bash
cargo test -- --nocapture
```

### 启用详细日志
```bash
RUST_LOG=debug cargo test -- --nocapture
```

### 只编译不运行
```bash
cargo test --no-run
```

### 串行运行（如果遇到并发问题）
```bash
cargo test -- --test-threads=1
```

## 测试标记

### `#[ignore]` 测试
这些测试需要 GPU 硬件，默认情况下会被跳过。

运行被忽略的测试：
```bash
cargo test -- --ignored
```

## 可用的测试

### 1. GPU 签名测试（不需要 GPU）
```bash
cargo test test_gpu_signature
```
这个测试可以运行，即使没有 GPU 也会返回 0。

### 2. FFI 函数测试（不需要 GPU）
```bash
cargo test test_ffi_functions
```

### 3. 适配器枚举测试（需要 GPU）
```bash
cargo test -- --ignored
```
这个测试需要实际的 GPU 硬件。

## 故障排除

### 问题 1：缺少 libclang

**错误信息**：
```
Unable to find libclang: "couldn't find any valid shared libraries matching: ['clang.dll', 'libclang.dll']"
```

**原因**：
项目的 `build.rs` 使用 `bindgen` 来生成 C++ 头文件的 Rust FFI 绑定。`bindgen` 需要 `libclang.dll` 来解析 C++ 代码。

**解决方案**：

#### 方案 1：安装 LLVM（推荐）

1. **下载并安装 LLVM**：
   - 访问：https://github.com/llvm/llvm-project/releases
   - 下载 Windows 版本（例如：LLVM-17.0.0-win64.exe）
   - 安装到默认位置：`C:\Program Files\LLVM`

2. **设置环境变量**：
   ```powershell
   $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
   ```

3. **或者使用 Chocolatey**：
   ```powershell
   choco install llvm
   ```

#### 方案 2：使用 Visual Studio 的 LLVM

如果你安装了 Visual Studio 2022，它可能包含 LLVM：

```powershell
# 检查是否存在
$vsPath = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\bin"
if (Test-Path "$vsPath\libclang.dll") {
    $env:LIBCLANG_PATH = $vsPath
}
```

#### 方案 3：永久设置环境变量（可选）

```powershell
[System.Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")
```

#### 验证安装

```powershell
# 检查 libclang.dll 是否存在
Test-Path "C:\Program Files\LLVM\bin\libclang.dll"

# 搜索 libclang.dll（如果不知道安装路径）
Get-ChildItem -Path "C:\Program Files" -Filter "libclang.dll" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName

# 搜索 Visual Studio 的 LLVM
Get-ChildItem -Path "C:\Program Files\Microsoft Visual Studio" -Filter "libclang.dll" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
```

### 问题 2：编译错误

如果遇到编译错误，先尝试：
```bash
cargo clean
cargo build
```

### 问题 3：缺少 GPU

如果没有 GPU，某些测试会失败或返回 0。这是正常的。

### 问题 4：Windows 平台要求

所有测试仅在 Windows 平台可用。在其他平台会跳过。

## 已修复的编译问题

### 1. ComPtr 导入错误 ✅

**问题**：`windows::core::ComPtr` 在 windows-rs 0.62 中不存在

**解决方案**：在 windows-rs 0.62 中，COM 接口类型本身就是智能指针，不需要 `ComPtr` 包装。

**修复**：
- 将所有 `ComPtr<T>` 替换为直接使用 `T`
- 将 `ComPtr::from_raw()` 替换为 `T::from_raw()` 或 `assume_init()`
- 将 `.cast()` 替换为 `windows::core::Interface::cast()`
- 将 `.as_raw()` 替换为直接传递引用

### 2. 缺少 Windows Features ✅

**问题**：缺少 `Win32_Storage_FileSystem` 和 `Win32_System_IO`

**解决方案**：在 `Cargo.toml` 中添加了这些 features

### 3. 类型转换问题 ✅

**问题**：`D3D11_BIND_FLAG`、`D3D11_RESOURCE_MISC_FLAG`、`D3D11_CPU_ACCESS_FLAG` 等需要转换为 `u32`

**解决方案**：使用 `.0 as u32` 进行转换

**示例**：
```rust
// 修复前
BindFlags: D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET,

// 修复后
BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
```

### 4. 结构体字段访问 ✅

**问题**：`D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC` 和 `D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC` 的 `Texture2D` 字段在 `Anonymous` 中

**解决方案**：使用 `Anonymous.Texture2D` 访问

**示例**：
```rust
// 修复前
input_view_desc.Texture2D.MipSlice = 0;

// 修复后
input_view_desc.Anonymous.Texture2D.MipSlice = 0;
```

### 5. SemanticName 类型 ✅

**问题**：`D3D11_INPUT_ELEMENT_DESC` 的 `SemanticName` 需要 `PCSTR` 而不是 `HSTRING`

**解决方案**：使用 `PCSTR::from_raw()` 创建

**示例**：
```rust
// 修复前
SemanticName: windows::core::HSTRING::from("POSITION"),

// 修复后
SemanticName: PCSTR::from_raw(b"POSITION\0".as_ptr() as *const u8),
```

### 6. API 调用参数 ✅

**问题**：某些 Windows API 的参数类型不匹配

**解决方案**：
- `D3D11CreateDevice` 的第三个参数需要 `HMODULE::default()` 而不是 `None`
- 使用 `assume_init()` 从 `Option` 中提取值
- 直接传递引用而不是原始指针

## 测试输出示例

### 成功运行
```
running 2 tests
test platform::win::tests::test_gpu_signature ... ok
test tests::platform_win_tests::tests::test_ffi_functions ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 运行被忽略的测试
```
running 1 test
test platform::win::tests::test_adapter_enumeration ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 快速测试命令参考

```bash
# 1. 清理并重新构建
cargo clean
cargo build

# 2. 运行所有测试
cargo test

# 3. 运行并显示输出
cargo test -- --nocapture

# 4. 运行被忽略的测试
cargo test -- --ignored

# 5. 只编译不运行
cargo test --no-run

# 6. 运行特定测试
cargo test test_gpu_signature

# 7. 启用详细日志
RUST_LOG=debug cargo test -- --nocapture
```

## 注意事项

1. **内存管理**：在 windows-rs 中，接口类型自动管理引用计数，无需手动释放
2. **原始指针**：使用 `from_raw()` 时要注意所有权，必要时使用 `std::mem::forget()` 避免双重释放
3. **类型安全**：所有类型转换都使用 `unsafe` 块，确保在正确的上下文中使用
4. **平台限制**：所有测试仅在 Windows 平台可用
5. **GPU 要求**：某些测试需要 GPU 硬件，没有 GPU 时会返回 0（这是正常的）

## 下一步

1. **解决编译依赖**：安装 libclang（如果遇到相关错误）
2. **运行基础测试**：`cargo test test_gpu_signature`
3. **修复编译错误**：根据错误信息修复代码
4. **运行完整测试**：`cargo test`
5. **运行 GPU 测试**：`cargo test -- --ignored`（需要 GPU 硬件）
