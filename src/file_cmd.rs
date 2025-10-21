use crate::result::DetectResult;
use std::process::{Command, Stdio};
use std::io::Write;

pub struct FileCmd {
    enabled: bool,
}

impl FileCmd {
    pub fn new(enabled: bool) -> Option<Self> {
        if enabled { Some(Self { enabled }) } else { None }
    }

    pub fn data2result(&mut self, data: &[u8]) -> Option<DetectResult> {
        const MIN_DATA_SIZE: usize = 5;
        const IGNORES: &[&str] = &[
            "data",
            "empty",
            "Sendmail frozen configuration",
            "DBase 3 data file",
            "DOS executable",
            "Dyalog APL",
            "8086 relocatable",
            "SysEx File",
            "COM executable",
            "Non-ISO extended-ASCII text",
            "ISO-8859 text",
            "very short file",
            "International EBCDIC text",
            "lif file",
            "AmigaOS bitmap font",
            "a python script text executable",
        ];
        
        if !self.enabled || data.len() < MIN_DATA_SIZE { return None; }
        
        // 直接用 file -b 获取描述（不使用 --mime-type）
        let mut child = Command::new("file")
            .arg("-b")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(data);
        }

        let output = child.wait_with_output().ok()?;
        if !output.status.success() { return None; }

        let desc = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if desc.is_empty() { return None; }

        // 检查是否在忽略列表中（以这些字符串开头）
        for ignore in IGNORES {
            if desc.starts_with(ignore) {
                return None;
            }
        }

        Some(DetectResult::FileType(desc))
    }
}


