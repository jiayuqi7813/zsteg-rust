use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOrder {
    Lsb,
    Msb,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringsMode {
    First,
    All,
    Longest,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub verbose: i32,
    pub limit: usize,
    pub order: OrderSpec,
    pub step: usize,
    pub ystep: usize,
    pub channels: Option<Vec<String>>, // None means auto
    pub bits: Option<Vec<u16>>,        // Ruby: number or mask (>=0x100)
    pub bit_order: Option<BitOrder>,
    pub prime: PrimeSpec,
    pub pixel_align: PixelAlignSpec,
    pub shift: Option<usize>,
    pub invert: bool,
    pub file_cmd: bool,
    pub strings: Option<StringsMode>,
    pub min_str_len: usize,
    pub extra_checks: bool,
    pub zlib_flag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimeSpec { None, Only, All }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PixelAlignSpec { None, Only, All }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderSpec {
    Auto,
    All,
    Explicit(Vec<String>),
}

impl Default for Options {
    fn default() -> Self {
        Self {
            verbose: 0,
            limit: 256,
            order: OrderSpec::Auto,
            step: 1,
            ystep: 1,
            channels: None,
            bits: None,
            bit_order: None,
            prime: PrimeSpec::None,
            pixel_align: PixelAlignSpec::None,
            shift: None,
            invert: false,
            file_cmd: true,
            strings: None,
            min_str_len: 8,
            extra_checks: true,
            zlib_flag: false,
        }
    }
}

pub fn parse_bits(s: &str, pixel_align_out: &mut bool) -> Result<Vec<u16>> {
    let mut out: Vec<u16> = Vec::new();
    let mut s = s.trim().to_string();
    if let Some('p') = s.chars().last() {
        *pixel_align_out = true;
        s.pop();
    }
    let xs: Vec<&str> = s.split(',').collect();
    for x in xs {
        let x = x.trim();
        if x.eq_ignore_ascii_case("all") { // special: range 1-8
            out.extend(1u16..=8);
            continue;
        }
        if let Some(pos) = x.find('-') {
            let a = &x[..pos];
            let b = &x[pos + 1..];
            let aa = parse_bits_single(a)?;
            let bb = parse_bits_single(b)?;
            if aa == 0 || bb == 0 || aa > 8 || bb > 8 {
                bail!("invalid bits range: {}", x);
            }
            if aa <= bb { out.extend(aa..=bb); } else { out.extend(bb..=aa); }
        } else {
            out.push(parse_bits_single(x)?);
        }
    }
    out.dedup();
    Ok(out)
}

fn parse_bits_single(x: &str) -> Result<u16> {
    if x == "1" { // catch NOT A BINARY MASK early
        return Ok(1);
    }
    if let Some(hex) = x.strip_prefix("0x") {
        let v = u16::from_str_radix(hex, 16)?;
        return Ok(0x100 + (v & 0xff));
    }
    if let Some(bin) = x.strip_prefix("0b") {
        let v = u16::from_str_radix(bin, 2)?;
        return Ok(0x100 + (v & 0xff));
    }
    if x.chars().all(|c| c == '0' || c == '1') {
        let v = u16::from_str_radix(x, 2)?;
        return Ok(0x100 + (v & 0xff));
    }
    if let Ok(v) = x.parse::<u16>() { return Ok(v); }
    bail!("invalid bits value: {}", x)
}

pub fn decode_param_string(s: &str) -> Result<Options> {
    let mut o = Options::default();
    let mut pixel_align_flag = false;
    for x in s.split(',') {
        let x = x.trim();
        match x {
            "lsb" => o.bit_order = Some(BitOrder::Lsb),
            "msb" => o.bit_order = Some(BitOrder::Msb),
            "prime" => { o.prime = PrimeSpec::Only; o.extra_checks = false; },
            "zlib" => { o.zlib_flag = true; },
            _ => {
                // 尝试解析 bits: b1, b2, 1b, 2b 等格式
                // 但要排除 rgb, bgr 等通道名称
                if x.len() >= 2 {
                    if let Some(cap) = x.strip_prefix('b') {
                        // b1, b2, b3 等格式
                        if cap.chars().all(|c| c.is_ascii_digit() || c == 'p') && !cap.is_empty() {
                            if cap.ends_with('p') {
                                pixel_align_flag = true;
                                let num_part = &cap[..cap.len()-1];
                                o.bits = Some(parse_bits(num_part, &mut pixel_align_flag)?);
                            } else {
                                o.bits = Some(parse_bits(cap, &mut pixel_align_flag)?);
                            }
                            o.extra_checks = false;
                            continue;
                        }
                    }
                    if let Some(cap) = x.strip_suffix('b') {
                        // 1b, 2b 等格式
                        if cap.chars().all(|c| c.is_ascii_digit()) && !cap.is_empty() {
                            o.bits = Some(parse_bits(cap, &mut pixel_align_flag)?);
                            o.extra_checks = false;
                            continue;
                        }
                    }
                }
                
                if x.eq_ignore_ascii_case("xy") || x.eq_ignore_ascii_case("yx") ||
                   x.eq_ignore_ascii_case("yb") || x.eq_ignore_ascii_case("by") {
                    o.order = OrderSpec::Explicit(vec![x.to_string()]);
                    continue;
                }
                if x.chars().all(|c| matches!(c, 'r'|'g'|'b'|'a')) && !x.is_empty() {
                    o.channels = Some(vec![x.to_string()]);
                    o.extra_checks = false;
                    continue;
                }
                bail!("unknown param {}", x);
            }
        }
    }
    if pixel_align_flag { o.pixel_align = PixelAlignSpec::Only; }
    Ok(o)
}

pub fn merge_cli_into_options(base: &mut Options, cli: &crate::Cli) -> Result<()> {
    if cli.all {
        base.prime = PrimeSpec::All;
        base.order = OrderSpec::All;
        base.pixel_align = PixelAlignSpec::All;
        base.bits = Some((1u16..=8).collect());
        base.extra_checks = true; // 显式启用额外检查
    }
    if let Some(s) = &cli.extract { let _ = s; /* 占位：extract 模式下静音等 */ }
    if let Some(o) = &cli.order { base.order = OrderSpec::Explicit(o.split(',').map(|x| x.to_string()).collect()); }
    if let Some(c) = &cli.channels { base.channels = Some(c.split(',').map(|x| x.to_string()).collect()); base.extra_checks = false; }
    if let Some(b) = &cli.bits { let mut p = false; base.bits = Some(parse_bits(b, &mut p)?); if p { base.pixel_align = PixelAlignSpec::Only; } base.extra_checks = false; }
    if cli.lsb { base.bit_order = Some(BitOrder::Lsb); }
    if cli.msb { base.bit_order = Some(BitOrder::Msb); }
    if cli.prime { base.prime = PrimeSpec::Only; base.extra_checks = false; }
    if let Some(n) = cli.shift { base.shift = Some(n); }
    if let Some(n) = cli.step { base.step = n; }
    if cli.invert { base.invert = true; }
    if cli.pixel_align { base.pixel_align = PixelAlignSpec::Only; }
    if let Some(n) = cli.limit { base.limit = n; }
    base.file_cmd = cli.file_cmd;
    if cli.no_strings { base.strings = Some(StringsMode::None); }
    if let Some(s) = &cli.strings { base.strings = Some(match s.to_lowercase().as_str() { "first" => StringsMode::First, "all" => StringsMode::All, "longest" => StringsMode::Longest, "none"|"no" => StringsMode::None, _ => StringsMode::First }); }
    if let Some(n) = cli.min_str_len { base.min_str_len = n; }
    base.verbose = (cli.verbose as i32) - (cli.quiet as i32);
    Ok(())
}


