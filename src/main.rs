use clap::{ArgAction, Parser, ValueEnum};
use colored::*;
use std::path::PathBuf;

mod opts;
use opts::{decode_param_string, merge_cli_into_options, Options};
mod imageio;
mod checker;
mod extractor;
mod result;
mod hexdump;
mod file_cmd;

#[derive(Copy, Clone, Eq, PartialEq, Debug, ValueEnum)]
enum BitOrder { Lsb, Msb }

#[derive(Parser, Debug)]
#[command(name = "zsteg", version, about = "detect stegano-hidden data in PNG & BMP")] 
struct Cli {
    /// try all known methods
    #[arg(short = 'a', long = "all", action = ArgAction::SetTrue)]
    all: bool,

    /// extract specified payload, like '1b,rgb,lsb'
    #[arg(short = 'E', long = "extract")]
    extract: Option<String>,

    /// pixel iteration order: ALL,xy,yx,XY,YX,xY,Xy,bY,... (default: auto)
    #[arg(short = 'o', long = "order")]
    order: Option<String>,

    /// channels (R/G/B/A) or any combination, comma separated
    #[arg(short = 'c', long = "channels")]
    channels: Option<String>,

    /// number of bits, like 1 or '1,3,5' or range '1-8', supports mask '0x88'
    #[arg(short = 'b', long = "bits")]
    bits: Option<String>,

    /// least significant bit comes first
    #[arg(long = "lsb", action = ArgAction::SetTrue)]
    lsb: bool,

    /// most significant bit comes first
    #[arg(long = "msb", action = ArgAction::SetTrue)]
    msb: bool,

    /// analyze/extract only prime bytes/pixels
    #[arg(short = 'P', long = "prime", action = ArgAction::SetTrue)]
    prime: bool,

    /// prepend N zero bits
    #[arg(long = "shift")]
    shift: Option<usize>,

    /// step
    #[arg(long = "step")]
    step: Option<usize>,

    /// invert bits (XOR 0xff)
    #[arg(long = "invert", action = ArgAction::SetTrue)]
    invert: bool,

    /// pixel-align hidden data
    #[arg(long = "pixel-align", action = ArgAction::SetTrue)]
    pixel_align: bool,

    /// limit bytes checked, 0 = no limit (default: 256)
    #[arg(short = 'l', long = "limit")]
    limit: Option<usize>,

    /// use 'file' command to detect data type (default: YES)
    #[arg(long = "file", action = ArgAction::Set, default_value_t = true)]
    file_cmd: bool,

    /// disable ASCII strings finding (default: enabled)
    #[arg(long = "no-strings", action = ArgAction::SetTrue)]
    no_strings: bool,

    /// ASCII strings find mode: first, all, longest, none (default: first)
    #[arg(short = 's', long = "strings")]
    strings: Option<String>,

    /// minimum string length (default: 8)
    #[arg(short = 'n', long = "min-str-len")]
    min_str_len: Option<usize>,

    /// Run verbosely (can be used multiple times)
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    verbose: u8,

    /// Silent any warnings (can be used multiple times)
    #[arg(short = 'q', long = "quiet", action = ArgAction::Count)]
    quiet: u8,

    /// Force (or disable) color output (default: auto)
    #[arg(short = 'C', long = "color")]
    color: Option<bool>,

    /// filename.png [param_string]
    input: PathBuf,

    /// optional param_string shortcut like "2b,b,lsb,xy"
    param_string: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(force) = cli.color { colored::control::set_override(force); }

    // 解析选项：先默认，再合并 CLI，再合并 param_string（与 Ruby 行为一致：行内覆盖）
    let mut options = Options::default();
    merge_cli_into_options(&mut options, &cli)?;
    if let Some(ps) = &cli.param_string {
        let o2 = decode_param_string(ps)?;
        // 简单字段按优先级覆盖
        if o2.bit_order.is_some() { options.bit_order = o2.bit_order; }
        if o2.bits.is_some() { options.bits = o2.bits; }
        if !matches!(o2.order, opts::OrderSpec::Auto) { options.order = o2.order; }
        if o2.channels.is_some() { options.channels = o2.channels; }
        if !matches!(o2.pixel_align, opts::PixelAlignSpec::None) { options.pixel_align = o2.pixel_align; }
        if !matches!(o2.prime, opts::PrimeSpec::None) { options.prime = o2.prime; }
        if o2.zlib_flag { options.zlib_flag = true; }
        options.extra_checks &= o2.extra_checks; // 任一侧关闭则关闭
    }

    let img = imageio::Image::load(&cli.input)?;

    if let Some(expr) = &cli.extract {
        // 提取模式：处理特殊名称或解析参数字符串
        use std::io::Write;
        let data = if expr == "imagedata" {
            img.imagedata.clone()
        } else if expr.starts_with("chunk:") {
            // 格式: chunk:N 或 chunk:N:TYPE
            let parts: Vec<&str> = expr.split(':').collect();
            if parts.len() >= 2 {
                let idx: usize = parts[1].parse()?;
                img.chunks.get(idx).map(|c| c.data.clone()).unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            // 普通参数字符串，如 "b1,r,lsb,xy"
            let extract_opts = decode_param_string(expr)?;
            let mut final_opts = options.clone();
            if extract_opts.bit_order.is_some() { final_opts.bit_order = extract_opts.bit_order; }
            if extract_opts.bits.is_some() { final_opts.bits = extract_opts.bits; }
            if !matches!(extract_opts.order, opts::OrderSpec::Auto) { final_opts.order = extract_opts.order; }
            if extract_opts.channels.is_some() { final_opts.channels = extract_opts.channels; }
            if !matches!(extract_opts.prime, opts::PrimeSpec::None) { final_opts.prime = extract_opts.prime; }
            
            // 提取模式：如果 limit 是默认值，则使用无限制（与原版行为一致）
            if final_opts.limit == 256 { // DEFAULT_LIMIT
                final_opts.limit = 0; // 0 means no limit in extractor
            }
            
            let mut title = String::new();
            extractor::extract(&img, &final_opts, &mut title)
        };
        
        if options.zlib_flag {
            let mut dec = flate2::read::ZlibDecoder::new(&data[..]);
            let mut out = Vec::new();
            if let Ok(_) = std::io::Read::read_to_end(&mut dec, &mut out) { 
                print!("{}", String::from_utf8_lossy(&out)); 
            } else { 
                eprintln!("cannot decompress with zlib"); 
            }
        } else {
            std::io::stdout().write_all(&data)?;
        }
        return Ok(());
    }
    
    // 正常检查模式
    if options.verbose >= 0 {
        println!("{} {}", "[.]".green(), cli.input.display());
    }

    let c = checker::Checker::new(&img, &options);
    let _ = c.check();

    Ok(())
}
