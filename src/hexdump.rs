pub fn dump(prefix_spaces: usize, data: &[u8], limit: usize) -> String {
    let mut out = String::new();
    let limit = if limit == 0 { data.len() } else { data.len().min(limit) };
    let mut i = 0usize;
    while i < limit {
        let mut line = String::new();
        line.push_str(&" ".repeat(prefix_spaces));
        line.push_str(&format!("{:08x}: ", i));
        for j in 0..16 {
            if i + j < limit { line.push_str(&format!("{:02x} ", data[i + j])); } else { line.push_str("   "); }
        }
        line.push_str(" ");
        for j in 0..16 {
            if i + j < limit {
                let c = data[i + j];
                let ch = if c.is_ascii_graphic() || c == b' ' { c as char } else { '.' };
                line.push(ch);
            }
        }
        out.push_str(&line);
        out.push('\n');
        i += 16;
    }
    out
}


