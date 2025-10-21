use crate::imageio::Image;
use crate::opts::{BitOrder, Options, OrderSpec, PrimeSpec};

// 从像素按通道与位顺序提取比特，聚合为字节序列
pub fn extract(image: &Image, opts: &Options, title_out: &mut String) -> Vec<u8> {
	let bit_order = opts.bit_order.unwrap_or(BitOrder::Lsb);
	let channels_pattern = opts
		.channels
		.as_ref()
		.map(|v| v.join(""))
		.unwrap_or_else(|| "rgb".to_string());
	let (channels, per_channel_bits) = parse_channels_pattern(&channels_pattern);
	let bits = opts.bits.clone().unwrap_or_else(|| vec![1]);

    *title_out = format!(
        "b{},{},{},{}",
		if bits.len() == 1 { bits[0].to_string() } else { bits.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("/") },
		channels.iter().collect::<String>(),
        match bit_order { BitOrder::Lsb => "lsb", BitOrder::Msb => "msb" },
		order_string(opts)
	);

	let mut out: Vec<u8> = Vec::new();
	let mut cur: u8 = 0;
	let mut cur_bits: u8 = 0;
	
	// limit: 当输出达到 limit 字节时停止提取（原版行为）
	let limit = if opts.limit > 0 { opts.limit } else { usize::MAX };

	let pixels = &image.pixels; // RGBA8
	let has_alpha = true;
	let stride = if has_alpha { 4 } else { 3 }; // 实际此处固定 RGBA8

	match &opts.order {
		OrderSpec::Explicit(v) if v.iter().any(|s| s.contains('b') || s.contains('B')) => {
			// 字节迭代：从 imagedata (scanlines 原始字节) 中提取
			// 这是 WBStego 等工具使用的方式
			let imagedata = &image.imagedata;
			
			// 解析order：bY, bX, BY, BX, yb, YB, xb, XB
			let order_str = v.iter().find(|s| s.contains('b') || s.contains('B')).map(|s| s.as_str()).unwrap_or("bY");
			let (x_start, x_end, x_step, y_start, y_end, y_step) = parse_byte_order(order_str, image);
			
			let bytes_per_line = if image.format == crate::imageio::ImgFormat::Bmp {
				// BMP: 每行字节数 = width * bytes_per_pixel
				// 对于24位BMP = width * 3，但需要考虑行对齐
				((image.width as usize * 3 + 3) / 4) * 4
			} else {
				// PNG: imagedata 包含 scanline 数据，每行前有filter字节
				(image.width as usize * 4) + 1 // RGBA + 1 filter byte
			};
			
			// 按照 Ruby byte_iterator 的逻辑：
			// if type[0,1].downcase == 'b' => ROW iterator (natural): y0.step(y1,ystep){ |y| x0.step(x1,xstep){ |x| yield(x,y) }}
			// else => COLUMN iterator: x0.step(x1,xstep){ |x| y0.step(y1,ystep){ |y| yield(x,y) }}
			
			let is_row_first = order_str.chars().next().map(|c| c == 'b' || c == 'B').unwrap_or(true);
			
			let mut byte_idx = 0;
			
			if is_row_first {
				// ROW iterator: 先遍历行(y)，再遍历字节(x)
				'outer: for y in make_range(y_start, y_end, y_step) {
					let line_start = y * bytes_per_line;
					for x in make_range(x_start, x_end, x_step) {
						let pos = line_start + x;
						if pos >= imagedata.len() { break 'outer; }
						
						if use_prime(byte_idx, opts) {
							let value = imagedata[pos];
							push_bits(&mut out, &mut cur, &mut cur_bits, value, &bits, bit_order);
							if out.len() >= limit { break 'outer; }
						}
						byte_idx += 1;
					}
				}
			} else {
				// COLUMN iterator: 先遍历字节(x)，再遍历行(y)
				'outer: for x in make_range(x_start, x_end, x_step) {
					for y in make_range(y_start, y_end, y_step) {
						let line_start = y * bytes_per_line;
						let pos = line_start + x;
						if pos >= imagedata.len() { break 'outer; }
						
						if use_prime(byte_idx, opts) {
							let value = imagedata[pos];
							push_bits(&mut out, &mut cur, &mut cur_bits, value, &bits, bit_order);
							if out.len() >= limit { break 'outer; }
						}
						byte_idx += 1;
					}
				}
			}
		}
		OrderSpec::Explicit(v) if v.iter().any(|s| s.eq_ignore_ascii_case("yx")) => {
			'outer: for x in 0..image.width as usize {
				for y in 0..image.height as usize {
					let idx = (y * image.width as usize + x) * stride;
					process_pixel(idx, pixels, &channels, &per_channel_bits, &bits, bit_order, &mut cur, &mut cur_bits, &mut out, use_prime_pixel(y, x, opts));
					if out.len() >= limit { break 'outer; }
				}
			}
		}
		_ => {
			// 默认 xy
			'outer: for y in 0..image.height as usize {
				for x in 0..image.width as usize {
					let idx = (y * image.width as usize + x) * stride;
					process_pixel(idx, pixels, &channels, &per_channel_bits, &bits, bit_order, &mut cur, &mut cur_bits, &mut out, use_prime_pixel(y, x, opts));
					if out.len() >= limit { break 'outer; }
				}
			}
		}
	}

	// shift: 前置 N 个 0 比特（等价于将输出整体右移 N 位）
	if let Some(shift) = opts.shift { if shift > 0 { out = shift_prepend_zero_bits(out, shift as u32); } }
	// invert: 对每个字节异或 0xFF
	if opts.invert { for b in &mut out { *b ^= 0xFF; } }
	out
}

fn order_string(opts: &Options) -> String {
	match &opts.order {
		OrderSpec::Auto => "auto".to_string(),
		OrderSpec::All => "ALL".to_string(),
		OrderSpec::Explicit(v) => {
			if v.iter().any(|s| s.eq_ignore_ascii_case("yx")) { 
				"yx".to_string()
			} else if let Some(s) = v.iter().find(|s| s.contains('b') || s.contains('B')) { 
				s.clone()
			} else { 
				"xy".to_string()
			}
		}
	}
}

fn make_range(start: usize, end: usize, step: usize) -> Box<dyn Iterator<Item = usize>> {
	if start <= end {
		Box::new((start..=end).step_by(step))
	} else {
		Box::new((end..=start).rev().step_by(step))
	}
}

fn parse_byte_order(order: &str, image: &Image) -> (usize, usize, usize, usize, usize, usize) {
	// 解析类似 "bY", "BY", "yb", "Yb" 等
	// 'b'/'B' 表示字节（x方向），'y'/'Y' 表示行（y方向）
	// 小写表示正向（0->max），大写表示逆向（max->0）
	
	let bytes_per_line = if image.format == crate::imageio::ImgFormat::Bmp {
		((image.width as usize * 3 + 3) / 4) * 4
	} else {
		(image.width as usize * 4) + 1
	};
	
	let max_y = (image.height as usize).saturating_sub(1);
	let max_x = bytes_per_line.saturating_sub(1);
	
	// 默认值 (正向)
	let mut x_start = 0;
	let mut x_end = max_x;
	let x_step = 1;
	let mut y_start = 0;
	let mut y_end = max_y;
	let y_step = 1;
	
	for ch in order.chars() {
		match ch {
			'b' => { x_start = 0; x_end = max_x; }, // 正向
			'B' => { x_start = max_x; x_end = 0; }, // 逆向
			'y' => { y_start = 0; y_end = max_y; }, // 正向
			'Y' => { y_start = max_y; y_end = 0; }, // 逆向
			_ => {}
		}
	}
	
	(x_start, x_end, x_step, y_start, y_end, y_step)
}

fn parse_channels_pattern(p: &str) -> (Vec<char>, Vec<u8>) {
	// 支持 "rgb" 或 "r3g2b3" 形式
	let chars: Vec<char> = p.chars().collect();
	if chars.len() % 2 == 0 && chars.chunks(2).all(|c| matches!(c[0], 'r'|'g'|'b'|'a') && c[1].is_ascii_digit()) {
		let mut chs = Vec::new();
		let mut bits = Vec::new();
		for i in (0..chars.len()).step_by(2) {
			chs.push(chars[i]);
			bits.push((chars[i+1] as u8 - b'0') as u8);
		}
		(chs, bits)
	} else {
		(chars, vec![])
	}
}

fn process_pixel(idx: usize, pixels: &[u8], channels: &[char], per_channel_bits: &[u8], bits: &[u16], bit_order: BitOrder, cur: &mut u8, cur_bits: &mut u8, out: &mut Vec<u8>, use_it: bool) {
	if !use_it { return; }
	for (i, ch) in channels.iter().enumerate() {
		let v = match *ch {
			'r' => pixels[idx],
			'g' => pixels[idx + 1],
			'b' => pixels[idx + 2],
			'a' => pixels[idx + 3],
			_ => 0,
		};
		if !per_channel_bits.is_empty() {
			let nb = per_channel_bits.get(i).copied().unwrap_or(1) as u16;
			push_nbits(out, cur, cur_bits, v, nb, bit_order);
		} else {
			for &nb in bits {
				let nb = if nb <= 8 { nb } else { (nb & 0xff).count_ones() as u16 };
				push_nbits(out, cur, cur_bits, v, nb, bit_order);
			}
		}
	}
}

fn push_bits(out: &mut Vec<u8>, cur: &mut u8, cur_bits: &mut u8, v: u8, bits: &[u16], bit_order: BitOrder) {
	for &nb in bits {
		let nb = if nb <= 8 { nb } else { (nb & 0xff).count_ones() as u16 };
		push_nbits(out, cur, cur_bits, v, nb, bit_order);
	}
}

fn push_nbits(out: &mut Vec<u8>, cur: &mut u8, cur_bits: &mut u8, v: u8, nb: u16, bit_order: BitOrder) {
	// 原版逻辑：逐位提取，然后按 bit_order 排列成字节
	// bit_indexes(bits) 返回要提取的位索引，例如 bits=3 返回 [2,1,0]
	let nb_u = (nb as u8).min(8);
	let bit_indexes: Vec<u8> = (0..nb_u).rev().collect(); // [nb_u-1, ..., 1, 0]
	
	// 临时存储提取的位
	let mut bits = Vec::new();
	for bidx in bit_indexes {
		let bit_value = (v >> bidx) & 1;
		bits.push(bit_value);
	}
	
	// 按 bit_order 将位累积成字节
	for bit in bits {
		match bit_order {
			BitOrder::Lsb => {
				// lsb: 第一个位放在最高位
				*cur = (*cur << 1) | bit;
			}
			BitOrder::Msb => {
				// msb: 第一个位放在最低位
				*cur |= bit << *cur_bits;
			}
		}
		*cur_bits += 1;
		
		if *cur_bits >= 8 {
			out.push(*cur);
			*cur = 0;
			*cur_bits = 0;
		}
	}
}

fn use_prime(byte_index: usize, opts: &Options) -> bool {
	match opts.prime {
		PrimeSpec::None => true,
		PrimeSpec::Only => is_prime(byte_index as u64),
		PrimeSpec::All => true,
	}
}

fn use_prime_pixel(y: usize, x: usize, opts: &Options) -> bool {
	match opts.prime {
		PrimeSpec::None => true,
		PrimeSpec::Only => is_prime((y as u64) * 1_000_003 + (x as u64)),
		PrimeSpec::All => true,
	}
}

fn is_prime(n: u64) -> bool {
	if n < 2 { return false; }
	if n % 2 == 0 { return n == 2; }
	let mut d = 3;
	while d * d <= n { if n % d == 0 { return false; } d += 2; }
	true
}

fn shift_prepend_zero_bits(mut data: Vec<u8>, shift_bits: u32) -> Vec<u8> {
	let r = (shift_bits % 8) as u8;
	if r == 0 { return data; }
	let mut carry = 0u8;
	for b in data.iter_mut().rev() {
		let new_carry = *b & ((1 << r) - 1);
		*b = (*b >> r) | (carry << (8 - r));
		carry = new_carry;
	}
	data
}


