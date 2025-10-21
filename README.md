# zsteg-rust

完全用 Rust 重构的 zsteg，功能与 [原版 Ruby zsteg](https://github.com/zed-0xff/zsteg) 几乎完全一致（缺什么功能发issue进行补充）。

## 编译与运行

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

#
