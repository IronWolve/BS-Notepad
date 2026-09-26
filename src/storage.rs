use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read, Write},
    path::Path,
};

pub const PREVIEW_BYTES: usize = 256 * 1024;
pub const MAX_EDIT_BYTES: usize = 32 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TextFormat {
    pub encoding: String,
    pub ending: String,
}
impl Default for TextFormat {
    fn default() -> Self {
        Self {
            encoding: "UTF-8".into(),
            ending: "LF".into(),
        }
    }
}
impl TextFormat {
    pub fn from_text(text: &str) -> Self {
        Self {
            encoding: if text.starts_with('\u{feff}') {
                "UTF-8 BOM"
            } else {
                "UTF-8"
            }
            .into(),
            ending: if text.contains("\r\n") {
                "CRLF"
            } else if text.contains('\r') {
                "CR"
            } else {
                "LF"
            }
            .into(),
        }
    }
    pub fn encode(&self, text: &str) -> Vec<u8> {
        let text = match self.ending.as_str() {
            "CRLF" => text.replace('\n', "\r\n"),
            "CR" => text.replace('\n', "\r"),
            _ => text.to_string(),
        };
        match self.encoding.as_str() {
            "UTF-16 LE" | "UTF-16 BE" => {
                let le = self.encoding == "UTF-16 LE";
                let mut out = if le {
                    vec![0xff, 0xfe]
                } else {
                    vec![0xfe, 0xff]
                };
                for c in text.encode_utf16() {
                    out.extend(if le { c.to_le_bytes() } else { c.to_be_bytes() });
                }
                out
            }
            "UTF-8 BOM" => {
                let mut out = vec![0xef, 0xbb, 0xbf];
                out.extend(text.as_bytes());
                out
            }
            _ => text.into_bytes(),
        }
    }
}
pub fn normalize(text: &str) -> String {
    text.trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}
pub fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
    })
}
#[derive(Clone, Debug)]
pub struct Loaded {
    pub modified: Option<std::time::SystemTime>,
    pub source: String,
    pub format: TextFormat,
    pub fingerprint: u64,
    pub read_only: bool,
}
pub fn read(path: &Path, limit_mb: u32) -> io::Result<Loaded> {
    let mut file = std::fs::File::open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(io::Error::other("Only regular text files can be opened."));
    }
    let limit = (limit_mb as usize * 1024 * 1024).min(MAX_EDIT_BYTES);
    let read_only = meta.len() > limit as u64;
    let cap = if read_only { PREVIEW_BYTES } else { limit + 1 };
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(cap as u64)
        .read_to_end(&mut bytes)?;
    let read_only = read_only || bytes.len() > limit;
    if read_only {
        bytes.truncate(PREVIEW_BYTES);
    }
    let fingerprint = fingerprint(&bytes);
    let (text, encoding) = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let le = bytes[0] == 0xff;
        let data = &bytes[2..];
        if data.len() % 2 != 0 && !read_only {
            return Err(io::Error::other("Incomplete UTF-16 character."));
        }
        let words: Vec<u16> = data
            .chunks_exact(2)
            .map(|c| {
                if le {
                    u16::from_le_bytes([c[0], c[1]])
                } else {
                    u16::from_be_bytes([c[0], c[1]])
                }
            })
            .collect();
        let text = if read_only {
            String::from_utf16_lossy(&words)
        } else {
            String::from_utf16(&words).map_err(|_| io::Error::other("Invalid UTF-16 text."))?
        };
        (text, if le { "UTF-16 LE" } else { "UTF-16 BE" })
    } else {
        let bom = bytes.starts_with(&[0xef, 0xbb, 0xbf]);
        let data = if bom { &bytes[3..] } else { &bytes[..] };
        let text = match std::str::from_utf8(data) {
            Ok(text) => text.to_string(),
            Err(e) if read_only && e.error_len().is_none() => {
                String::from_utf8_lossy(&data[..e.valid_up_to()]).into_owned()
            }
            Err(_) => {
                return Err(io::Error::other(
                    "Unsupported encoding. Use UTF-8 or UTF-16 with a byte-order mark.",
                ))
            }
        };
        if text.contains('\0') {
            return Err(io::Error::other("This appears to be a binary file."));
        }
        (text, if bom { "UTF-8 BOM" } else { "UTF-8" })
    };
    let mut format = TextFormat::from_text(&text);
    format.encoding = encoding.into();
    Ok(Loaded {
        modified: meta.modified().ok(),
        source: normalize(&text),
        format,
        fingerprint,
        read_only,
    })
}
pub fn disk_fingerprint(path: &Path) -> io::Result<u64> {
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > MAX_EDIT_BYTES as u64 {
        return Err(io::Error::other("File now exceeds the editing limit."));
    }
    let mut data = Vec::new();
    file.take(MAX_EDIT_BYTES as u64 + 1)
        .read_to_end(&mut data)?;
    Ok(fingerprint(&data))
}
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let filename = path
        .file_name()
        .ok_or_else(|| io::Error::other("No filename"))?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = path.with_file_name(format!(
        ".{}.{}.{}.tmp",
        filename.to_string_lossy(),
        std::process::id(),
        nonce
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        if let Ok(meta) = std::fs::metadata(path) {
            file.set_permissions(meta.permissions())?;
        }
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)?;
        #[cfg(unix)]
        if let Some(parent) = path.parent() {
            std::fs::File::open(parent)?.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

pub fn apply_patch(text: &mut String, start: usize, end: usize, insert: &str) -> bool {
    fn byte_at(text: &str, index: usize) -> Option<usize> {
        let mut units = 0;
        for (offset, c) in text.char_indices() {
            if units == index {
                return Some(offset);
            }
            units += c.len_utf16();
            if units > index {
                return None;
            }
        }
        (units == index).then_some(text.len())
    }
    if start > end {
        return false;
    }
    let (Some(a), Some(b)) = (byte_at(text, start), byte_at(text, end)) else {
        return false;
    };
    text.replace_range(a..b, insert);
    true
}
