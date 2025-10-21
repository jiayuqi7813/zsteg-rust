# zsteg-rust

[![CI](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/ci.yml)
[![Release](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/release.yml/badge.svg)](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/release.yml)

完全用 Rust 重构的 zsteg，功能与 [原版 Ruby zsteg](https://github.com/zed-0xff/zsteg) 几乎完全一致（缺什么功能发issue进行补充）。

## 安装

### 从 Release 下载

前往 [Releases](https://github.com/jiayuqi7813/zsteg-rust/releases) 页面下载适合你平台的预编译二进制文件：

- **Linux**: `zsteg-rust-linux-x86_64` 或 `zsteg-rust-linux-aarch64`
- **macOS**: `zsteg-rust-macos-x86_64` 或 `zsteg-rust-macos-aarch64` (Apple Silicon)
- **Windows**: `zsteg-rust-windows-x86_64.exe`

下载后添加执行权限（Linux/macOS）：
```bash
chmod +x zsteg-rust-*
```

### 从源码编译

```bash
cd zsteg-rust
cargo build --release
./target/release/zsteg-rust <image.png>
```

## 使用

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

- ✅ LSB 隐写检测（PNG/BMP）
- ✅ zlib 压缩数据检测
- ✅ 多通道与位组合扫描
- ✅ 像素顺序（xy/yx/bY）
- ✅ 质数位置提取（--prime）
- ✅ file 命令集成
- ✅ 数据去重与缓存
- ✅ 完整 CLI 参数对齐
- ✅ 修复了原版wbsteg解密报错的bug。 



## 示例

```bash
❯ ./target/release/zsteg-rust ../zsteg/samples/flower_rgb3.png
imagedata           .. file: 370 XA sysV pure executable not stripped - version 768
b3,rgb,lsb,xy       .. text: "SuperSecretMessage"
```

与原版输出完全一致。

## 开发

### 构建流水线

项目使用 GitHub Actions 进行自动化构建和发布：

- **CI** (`ci.yml`): 每次推送到主分支或 PR 时运行测试和构建
- **Release** (`release.yml`): 创建 tag (如 `v1.0.0`) 时自动构建并发布多平台二进制文件
- **Nightly** (`nightly.yml`): 每天自动构建最新开发版本

### 发布新版本

1. 更新版本号并创建 tag：
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. GitHub Actions 会自动：
   - 构建所有平台的二进制文件
   - 生成 SHA256 校验和
   - 创建 GitHub Release
   - 上传所有构建产物

### 支持的平台

- Linux x86_64 / ARM64
- macOS x86_64 / ARM64 (Apple Silicon)
- Windows x86_64
