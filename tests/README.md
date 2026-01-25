# 测试说明

## 运行测试

### 运行所有测试
```bash
cargo test
```

### 运行特定测试
```bash
cargo test test_gpu_signature
```

### 运行被忽略的测试（需要 GPU）
```bash
cargo test -- --ignored
```

### 运行所有测试（包括被忽略的）
```bash
cargo test -- --include-ignored
```

## 测试分类

### 1. 单元测试
位置：`src/platform/win/mod.rs` 中的 `#[cfg(test)]` 模块

这些测试测试单个模块的功能。

### 2. 集成测试
位置：`tests/platform_win_tests.rs`

这些测试测试多个模块的集成。

### 3. 需要 GPU 的测试
标记为 `#[ignore]` 的测试需要实际的 GPU 硬件。

运行这些测试：
```bash
cargo test -- --ignored
```

## 注意事项

1. **GPU 要求**：某些测试需要可用的 GPU 硬件
2. **Windows 平台**：所有测试仅在 Windows 平台可用
3. **编译错误**：如果遇到编译错误，可能需要先修复代码

## 测试环境设置

### 启用日志
```bash
RUST_LOG=debug cargo test
```

### 显示测试输出
```bash
cargo test -- --nocapture
```

### 并行运行
默认情况下，测试会并行运行。如果遇到问题，可以串行运行：
```bash
cargo test -- --test-threads=1
```
