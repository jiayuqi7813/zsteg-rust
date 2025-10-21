use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImgFormat { Png, Bmp, Unknown }

#[derive(Debug, Clone)]
pub struct PngChunk {
    pub ty: [u8;4],
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Image {
    pub format: ImgFormat,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA8 (decoded pixels)
    pub imagedata: Vec<u8>, // raw scanlines with filter bytes (PNG) or pixels (BMP)
    pub metadata: HashMap<String, String>,
    pub chunks: Vec<PngChunk>, // PNG only; others empty
    pub extradata: Vec<Vec<u8>>, // 占位
}

impl Image {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let ext = path_ref.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "png" => Self::load_png(path_ref),
            "bmp" => Self::load_bmp(path_ref),
            _ => Self::load_auto(path_ref),
        }
    }

    fn load_auto(path: &Path) -> Result<Self> {
        // 简易魔数检测 PNG，否则尝试 BMP
        let mut f = File::open(path).with_context(|| format!("open {:?}", path))?;
        let mut sig = [0u8; 8];
        let n = f.read(&mut sig)?;
        drop(f);
        if n >= 8 && &sig == b"\x89PNG\r\n\x1a\n" {
            return Self::load_png(path);
        }
        Self::load_bmp(path)
    }

    fn load_bmp(path: &Path) -> Result<Self> {
        let img = image::open(path).with_context(|| "decode bmp")?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let pixels = rgba.to_vec();
        
        // 对于 BMP，imagedata 需要是原始的像素数据（BGR格式，带行填充）
        // 手动读取BMP文件的像素数据部分
        let imagedata = Self::read_bmp_pixel_data(path, w, h)?;
        
        Ok(Self {
            format: ImgFormat::Bmp,
            width: w,
            height: h,
            imagedata,
            pixels,
            metadata: HashMap::new(),
            chunks: Vec::new(),
            extradata: Vec::new(),
        })
    }
    
    fn read_bmp_pixel_data(path: &Path, width: u32, height: u32) -> Result<Vec<u8>> {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = File::open(path)?;
        
        // 读取 BMP 头部
        let mut header = [0u8; 14];
        f.read_exact(&mut header)?;
        
        // 检查签名
        if &header[0..2] != b"BM" {
            return Err(anyhow::anyhow!("Not a BMP file"));
        }
        
        // 读取 pixel data 的偏移量（字节10-13）
        let pixel_offset = u32::from_le_bytes([header[10], header[11], header[12], header[13]]);
        
        // 读取 DIB 头部
        let mut dib_header = [0u8; 40];
        f.read_exact(&mut dib_header)?;
        
        let bits_per_pixel = u16::from_le_bytes([dib_header[14], dib_header[15]]);
        // height 可能是负数，表示 top-down
        let height_signed = i32::from_le_bytes([dib_header[8], dib_header[9], dib_header[10], dib_header[11]]);
        let is_bottom_up = height_signed > 0;
        
        // 跳转到像素数据
        f.seek(SeekFrom::Start(pixel_offset as u64))?;
        
        // 计算每行字节数（需要4字节对齐）
        let bytes_per_pixel = (bits_per_pixel / 8) as usize;
        let row_size = ((width as usize * bytes_per_pixel + 3) / 4) * 4;
        let total_size = row_size * height as usize;
        
        // 读取所有像素数据
        let mut pixel_data = vec![0u8; total_size];
        f.read_exact(&mut pixel_data)?;
        
        // BMP 默认是 bottom-up 存储，需要翻转到 top-down
        // 因为 image crate 解码后的 RGBA 是 top-down 的
        if is_bottom_up {
            let mut flipped = vec![0u8; total_size];
            for y in 0..height as usize {
                let src_offset = y * row_size;
                let dst_offset = (height as usize - 1 - y) * row_size;
                flipped[dst_offset..dst_offset + row_size].copy_from_slice(&pixel_data[src_offset..src_offset + row_size]);
            }
            pixel_data = flipped;
        }
        
        Ok(pixel_data)
    }

    fn load_png(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("open {:?}", path))?;
        let mut r = BufReader::new(file);

        // 解析 PNG chunk 简单列表
        let mut sig = [0u8;8];
        r.read_exact(&mut sig)?;
        anyhow::ensure!(&sig == b"\x89PNG\r\n\x1a\n", "invalid png signature");

        let mut chunks: Vec<PngChunk> = Vec::new();
        let mut found_iend = false;
        loop {
            let mut len_buf = [0u8;4];
            if r.read(&mut len_buf)? < 4 { break; }
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut ty = [0u8;4];
            r.read_exact(&mut ty)?;
            let mut data = vec![0u8; len];
            if len > 0 { r.read_exact(&mut data)?; }
            let mut crc = [0u8;4];
            r.read_exact(&mut crc)?;
            chunks.push(PngChunk { ty, data });
            if &ty == b"IEND" { 
                found_iend = true;
                break; 
            }
        }

        // 提取IEND后的extradata
        let mut extradata = Vec::new();
        if found_iend {
            let mut extra_buf = Vec::new();
            let bytes_read = r.read_to_end(&mut extra_buf)?;
            if bytes_read > 0 {
                extradata.push(extra_buf);
            }
        }

        // 用 image crate 解码像素
        let img = image::open(path).with_context(|| "decode png")?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();

        // 提取常见文本元数据（tEXt/iTXt/zTXt 简易）
        let mut metadata = HashMap::new();
        for ch in &chunks {
            match &ch.ty {
                b"tEXt" => {
                    if let Some((k,v)) = parse_png_text(&ch.data) { metadata.insert(k, v); }
                }
                b"iTXt" => {
                    if let Some((k,v)) = parse_png_itxt(&ch.data) { metadata.insert(k, v); }
                }
                b"zTXt" => {
                    if let Some((k,v)) = parse_png_ztxt(&ch.data) { metadata.insert(k, v); }
                }
                _ => {}
            }
        }

        // 提取 imagedata：解压所有 IDAT 块得到原始扫描线数据
        let mut idat_data = Vec::new();
        for ch in &chunks {
            if &ch.ty == b"IDAT" {
                idat_data.extend_from_slice(&ch.data);
            }
        }
        
        // 解压 zlib 数据得到原始扫描线
        use flate2::read::ZlibDecoder;
        use std::io::Read;
        let mut decoder = ZlibDecoder::new(&idat_data[..]);
        let mut imagedata = Vec::new();
        decoder.read_to_end(&mut imagedata).with_context(|| "decompress IDAT")?;

        Ok(Self {
            format: ImgFormat::Png,
            width: w,
            height: h,
            pixels: rgba.to_vec(),
            imagedata,
            metadata,
            chunks,
            extradata,
        })
    }
}

fn parse_png_text(data: &[u8]) -> Option<(String,String)> {
    // tEXt: keyword\0text
    let pos = data.iter().position(|&b| b == 0)?;
    let (k, v) = data.split_at(pos);
    let v = &v[1..];
    Some((String::from_utf8_lossy(k).to_string(), String::from_utf8_lossy(v).to_string()))
}

fn parse_png_itxt(data: &[u8]) -> Option<(String,String)> {
    // iTXt: keyword\0compressFlag\0langTag\0translated\0text (简化：忽略压缩与语言)
    let mut parts = data.split(|&b| b == 0);
    let k = parts.next()?;
    let _compress = parts.next()?; // flag+method（简化合并）
    let _lang = parts.next()?;
    let _translated = parts.next()?;
    let rest = parts.next().unwrap_or_default();
    Some((String::from_utf8_lossy(k).to_string(), String::from_utf8_lossy(rest).to_string()))
}

fn parse_png_ztxt(data: &[u8]) -> Option<(String,String)> {
    // zTXt: keyword\0compressionMethod\0compressedText
    let pos = data.iter().position(|&b| b == 0)?;
    let k = &data[..pos];
    if pos + 2 > data.len() { return None; }
    let _compression_method = data[pos + 1]; // 应该是 0 (deflate)
    let compressed_data = &data[pos + 2..];
    
    // 解压数据
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    let mut decoder = ZlibDecoder::new(compressed_data);
    let mut decompressed = Vec::new();
    if decoder.read_to_end(&mut decompressed).is_err() {
        return None;
    }
    
    Some((String::from_utf8_lossy(k).to_string(), String::from_utf8_lossy(&decompressed).to_string()))
}


