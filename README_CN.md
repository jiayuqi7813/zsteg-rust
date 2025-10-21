# zsteg-rust

[![CI](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/ci.yml)
[![Release](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/release.yml/badge.svg)](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/release.yml)

zsteg的完整Rust重写版本，功能与[原版Ruby zsteg](https://github.com/zed-0xff/zsteg)几乎完全相同（缺失功能可通过issues请求）。

[中文版 README](README_CN.md) | [English README](README.md)

## 安装

### 从发布页面下载

访问[发布页面](https://github.com/jiayuqi7813/zsteg-rust/releases)下载适合你平台的预编译二进制文件：

- **Linux**: `zsteg-rust-linux-x86_64` 或 `zsteg-rust-linux-aarch64`
- **macOS**: `zsteg-rust-macos-x86_64` 或 `zsteg-rust-macos-aarch64` (Apple Silicon)
- **Windows**: `zsteg-rust-windows-x86_64.exe`

下载后，添加执行权限（Linux/macOS）：
```bash
chmod +x zsteg-rust-*
```

### 从源码构建

```bash
cd zsteg-rust
cargo build --release
./target/release/zsteg-rust <image.png>
```

## 使用方法

基本用法与原版完全相同：

```bash
# 默认检查
./target/release/zsteg-rust image.png

# 尝试所有方法
./target/release/zsteg-rust -a image.png

# 提取数据
./target/release/zsteg-rust -E "1b,rgb,lsb" image.png > output.bin
```

## 功能对齐

- ✅ LSB隐写检测（PNG/BMP）
- ✅ zlib压缩数据检测
- ✅ 多通道与位组合扫描
- ✅ 像素顺序（xy/yx/bY）
- ✅ 质数位置提取（--prime）
- ✅ file命令集成
- ✅ 数据去重与缓存
- ✅ 完整CLI参数对齐
- ✅ 修复了原版wbsteg解密bug

## 示例

```bash
❯ ./target/release/zsteg-rust ../zsteg/samples/flower_rgb3.png
imagedata           .. file: 370 XA sysV pure executable not stripped - version 768
b3,rgb,lsb,xy       .. text: "SuperSecretMessage"
```

输出与原版完全相同。

## 开发

### 构建流水线

项目使用GitHub Actions进行自动化构建和发布：

- **CI** (`ci.yml`): 每次推送到main分支或PR时运行测试和构建
- **Release** (`release.yml`): 创建tag时自动构建和发布多平台二进制文件（如`v1.0.0`）
- **Nightly** (`nightly.yml`): 每日自动构建最新开发版本

### 发布新版本

1. 更新版本号并创建tag：
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. GitHub Actions将自动：
   - 为所有平台构建二进制文件
   - 生成SHA256校验和
   - 创建GitHub Release
   - 上传所有构建产物

### 支持的平台

- Linux x86_64 / ARM64
- macOS x86_64 / ARM64 (Apple Silicon)
- Windows x86_64
