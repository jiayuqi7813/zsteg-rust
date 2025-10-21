use std::fmt;
use colored::Colorize;

/// 将字节数组转换为带转义的字符串表示（类似 Ruby 的 inspect）
fn escape_bytes_to_string(data: &[u8]) -> String {
    let mut result = String::from("\"");
    for &b in data {
        match b {
            b'\n' => result.push_str("\\n"),
            b'\r' => result.push_str("\\r"),
            b'\t' => result.push_str("\\t"),
            b'\\' => result.push_str("\\\\"),
            b'"' => result.push_str("\\\""),
            0x07 => result.push_str("\\a"),  // bell
            0x08 => result.push_str("\\b"),  // backspace
            0x0B => result.push_str("\\v"),  // vertical tab
            0x0C => result.push_str("\\f"),  // form feed
            0x1B => result.push_str("\\e"),  // escape
            0x20..=0x7e => result.push(b as char), // 可打印ASCII
            _ => result.push_str(&format!("\\x{:02X}", b)), // 不可见字符用 hex
        }
    }
    result.push('"');
    result
}

#[derive(Debug, Clone)]
pub enum DetectResult {
    OneChar { ch: u8, size: usize },
    WholeText(String),
    PartialText { text: String, offset: usize },
    Zlib { data: Vec<u8>, offset: usize, size: usize },
    OpenStego { version: u8, data_len: u32, channel_bits: u8, fname_len: u8, compress: u8, encrypt: u8, fname: String },
    WBStego { size: u32, ext: Option<String>, data_preview: Vec<u8>, enc: Option<String>, even: bool },
    FileType(String),
}

impl fmt::Display for DetectResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DetectResult::OneChar { ch, size } => {
                // 单字符重复用灰色
                write!(f, "{}", format!("[0x{:02X}] repeated {} times", ch, size).bright_black())
            }
            DetectResult::WholeText(s) => {
                // WholeText: "text: " 灰色，内容亮红色
                write!(f, "{}{}", "text: ".bright_black(), format!("{:?}", s).bright_red())
            }
            DetectResult::PartialText { text, offset } => {
                // PartialText: 根据条件决定颜色
                let text_repr = format!("{:?}", text);
                let colored_text = if *offset == 0 {
                    // 从数据开头开始 => 亮红色
                    text_repr.bright_red().to_string()
                } else if text.len() > 10 && text.contains(' ') && 
                          text.chars().all(|c| c.is_ascii_alphanumeric() || " .,:!_-\r\n\t".contains(c)) {
                    // 长文本且包含空格，且是ASCII字符 => 亮红色
                    text_repr.bright_red().to_string()
                } else {
                    // 其他情况不加颜色
                    text_repr
                };
                write!(f, "{}{}", "text: ".bright_black(), colored_text)
            }
            DetectResult::Zlib { data, offset, size } => {
                // 限制预览大小为100字节（与原版一致）
                const MAX_SHOW_SIZE: usize = 100;
                let preview_data = &data[..data.len().min(MAX_SHOW_SIZE)];
                
                // 将数据转换为带转义的字符串表示（类似 Ruby 的 inspect）
                let preview = escape_bytes_to_string(preview_data);
                let suffix = if data.len() > MAX_SHOW_SIZE { "..." } else { "" };
                
                write!(f, "zlib: data={}{}, offset={}, size={}", 
                    preview.bright_red(), 
                    suffix,
                    offset, 
                    data.len())
            }
            DetectResult::OpenStego { version, data_len, channel_bits, fname_len, compress, encrypt, fname } => {
                let summary = format!(
                    "version={} data_len=0x{:x} channel_bits={} fname_len={} compress={} encrypt={} fname={:?}",
                    version, data_len, channel_bits, fname_len, compress, encrypt, fname
                );
                write!(f, "{}", format!("<ZSteg::Result::OpenStego {}", summary).bright_red())
            }
            DetectResult::WBStego { size, ext, data_preview, enc, even } => {
                let mut parts = vec![format!("size={}", size)];
                if let Some(e) = ext {
                    parts.push(format!("ext={:?}", e));
                }
                // 对原始字节进行转义处理
                let escaped_data = escape_bytes_to_string(data_preview);
                // escape_bytes_to_string 已经包含了引号，所以不需要再加
                parts.push(format!("data={}...", escaped_data));
                if *even {
                    parts.push("even=true".to_string());
                }
                if let Some(e) = enc {
                    parts.push(format!("enc={:?}", e));
                }
                
                let summary = format!("<wbStego {}>", parts.join(", "));
                // 根据是否有有效扩展名决定颜色
                if ext.is_some() && enc.is_none() {
                    write!(f, "{}", summary.bright_red())
                } else {
                    write!(f, "{}", summary.bright_black())
                }
            }
            DetectResult::FileType(desc) => {
                // file 命令结果的颜色逻辑
                let colored_desc = if desc.to_lowercase().contains("dbase 3 data") {
                    // DBase 3 data => 整体灰色
                    format!("file: {}", desc).bright_black().to_string()
                } else {
                    // 检查是否匹配特定关键词
                    let keywords = ["bitmap", "jpeg", "pdf", "zip", "rar", "7z", "7-z"];
                    let is_highlighted = desc.to_lowercase().split_whitespace()
                        .any(|word| keywords.iter().any(|kw| word.starts_with(kw)));
                    
                    if is_highlighted {
                        format!("file: {}", desc.bright_red())
                    } else {
                        format!("file: {}", desc.yellow())
                    }
                };
                write!(f, "{}", colored_desc)
            }
        }
    }
}


