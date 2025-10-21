# zsteg-rust

[![CI](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/ci.yml)
[![Release](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/release.yml/badge.svg)](https://github.com/jiayuqi7813/zsteg-rust/actions/workflows/release.yml)

A complete Rust rewrite of zsteg with functionality almost identical to the [original Ruby zsteg](https://github.com/zed-0xff/zsteg) (missing features can be requested via issues).

[中文版 README](README_CN.md) | [English README](README.md)


## Installation

### Download from Releases

Visit the [Releases](https://github.com/jiayuqi7813/zsteg-rust/releases) page to download pre-compiled binaries for your platform:

- **Linux**: `zsteg-rust-linux-x86_64` or `zsteg-rust-linux-aarch64`
- **macOS**: `zsteg-rust-macos-x86_64` or `zsteg-rust-macos-aarch64` (Apple Silicon)
- **Windows**: `zsteg-rust-windows-x86_64.exe`

After downloading, add execute permissions (Linux/macOS):
```bash
chmod +x zsteg-rust-*
```

### Build from Source

```bash
cd zsteg-rust
cargo build --release
./target/release/zsteg-rust <image.png>
```

## Usage

Basic usage is identical to the original:

```bash
# Default check
./target/release/zsteg-rust image.png

# Try all methods
./target/release/zsteg-rust -a image.png

# Extract data
./target/release/zsteg-rust -E "1b,rgb,lsb" image.png > output.bin
```

## Feature Alignment

- ✅ LSB steganography detection (PNG/BMP)
- ✅ zlib compressed data detection
- ✅ Multi-channel and bit combination scanning
- ✅ Pixel order (xy/yx/bY)
- ✅ Prime position extraction (--prime)
- ✅ file command integration
- ✅ Data deduplication and caching
- ✅ Complete CLI parameter alignment
- ✅ Fixed wbsteg decryption bug from original version

## Examples

```bash
❯ ./target/release/zsteg-rust ../zsteg/samples/flower_rgb3.png
imagedata           .. file: 370 XA sysV pure executable not stripped - version 768
b3,rgb,lsb,xy       .. text: "SuperSecretMessage"
```

Output is identical to the original version.

## Development

### Build Pipeline

The project uses GitHub Actions for automated building and releasing:

- **CI** (`ci.yml`): Runs tests and builds on every push to main branch or PR
- **Release** (`release.yml`): Automatically builds and releases multi-platform binaries when tags are created (e.g., `v1.0.0`)
- **Nightly** (`nightly.yml`): Automatically builds latest development version daily

### Releasing New Versions

1. Update version number and create tag:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. GitHub Actions will automatically:
   - Build binaries for all platforms
   - Generate SHA256 checksums
   - Create GitHub Release
   - Upload all build artifacts

### Supported Platforms

- Linux x86_64 / ARM64
- macOS x86_64 / ARM64 (Apple Silicon)
- Windows x86_64
