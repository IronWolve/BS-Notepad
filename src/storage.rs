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
    #[serde(default)]
    pub mixed: bool,
}
impl Default for TextFormat {
    fn default() -> Self {
        Self {
            encoding: "UTF-8".into(),
            ending: "LF".into(),
            mixed: false,
        }
    }
}
impl TextFormat {
    pub fn from_text(text: &str) -> Self {
        let crlf = text.matches("\r\n").count();
        let counts = [
            text.bytes().filter(|b| *b == b'\n').count() - crlf,
            crlf,
            text.bytes().filter(|b| *b == b'\r').count() - crlf,
        ];
        let mut preferred = 0;
        for index in 1..3 {
            if counts[index] > counts[preferred] {
                preferred = index;
            }
        }
        Self {
            encoding: if text.starts_with('\u{feff}') {
                "UTF-8 BOM"
            } else {
                "UTF-8"
            }
            .into(),
            ending: ["LF", "CRLF", "CR"][preferred].into(),
            mixed: counts.iter().filter(|count| **count > 0).count() > 1,
        }
    }
    pub fn label(&self) -> String {
        if self.mixed {
            format!("Mixed ({})", self.ending)
        } else {
            self.ending.clone()
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
    text.replace("\r\n", "\n").replace('\r', "\n")
}
pub fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
    })
}
#[derive(Clone, Debug, Serialize)]
pub struct ImageInfo {
    pub format: String,
    pub bytes: u64,
}

#[derive(Clone, Debug)]
pub struct Loaded {
    pub image: Option<ImageInfo>,
    pub modified: Option<std::time::SystemTime>,
    pub source: String,
    pub format: TextFormat,
    pub fingerprint: u64,
    pub read_only: bool,
}
fn open_regular(path: &Path) -> io::Result<(std::fs::File, std::fs::Metadata)> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(io::Error::other("Only regular text files can be opened."));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Refuse a FIFO even if the path is replaced between metadata and open.
        #[cfg(target_os = "linux")]
        options.custom_flags(0x800);
        #[cfg(target_os = "macos")]
        options.custom_flags(0x4);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(io::Error::other("Only regular text files can be opened."));
    }
    Ok((file, meta))
}
pub fn read(path: &Path, limit_mb: u32) -> io::Result<Loaded> {
    let (mut file, meta) = open_regular(path)?;
    if let Some(format) = crate::assets::image_type(path) {
        if meta.len() > crate::assets::MAX_IMAGE_BYTES {
            return Err(io::Error::other(
                "This image exceeds the 32 MB viewing limit.",
            ));
        }
        return Ok(Loaded {
            image: Some(ImageInfo {
                format: format.into(),
                bytes: meta.len(),
            }),
            modified: meta.modified().ok(),
            source: String::new(),
            format: TextFormat::default(),
            fingerprint: 0,
            read_only: true,
        });
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
    if bytes.starts_with(&[0xff, 0xfe, 0, 0]) || bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        return Err(io::Error::other(
            "UTF-32 is not supported. Reopen a UTF-8 or UTF-16 copy.",
        ));
    }
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
    if text.contains('\0') {
        return Err(io::Error::other("This appears to be a binary file."));
    }
    let mut format = TextFormat::from_text(&text);
    format.encoding = encoding.into();
    Ok(Loaded {
        image: None,
        modified: meta.modified().ok(),
        source: normalize(&text),
        format,
        fingerprint,
        read_only,
    })
}
pub fn disk_fingerprint(path: &Path) -> io::Result<u64> {
    let (file, meta) = open_regular(path)?;
    if meta.len() > MAX_EDIT_BYTES as u64 {
        return Err(io::Error::other("File now exceeds the editing limit."));
    }
    let mut data = Vec::new();
    file.take(MAX_EDIT_BYTES as u64 + 1)
        .read_to_end(&mut data)?;
    Ok(fingerprint(&data))
}
#[derive(Debug)]
pub struct WriteReport {
    pub warning: Option<String>,
}

fn sync_file(file: &std::fs::File) -> io::Result<()> {
    match file.sync_all() {
        Ok(()) => Ok(()),
        #[cfg(target_os = "macos")]
        Err(error)
            if error.kind() == io::ErrorKind::Unsupported
                || matches!(error.raw_os_error(), Some(22 | 45)) =>
        {
            use std::os::fd::AsRawFd;
            unsafe extern "C" {
                fn fsync(fd: i32) -> i32;
            }
            if unsafe { fsync(file.as_raw_fd()) } == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let report = write_report(path, bytes, false, |parent| {
        sync_file(&std::fs::File::open(parent)?)
    })?;
    if let Some(warning) = report.warning {
        crate::log::line(&warning);
    }
    Ok(())
}

pub fn write_private_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let report = write_report(path, bytes, true, |parent| {
        sync_file(&std::fs::File::open(parent)?)
    })?;
    if let Some(warning) = report.warning {
        crate::log::line(&warning);
    }
    Ok(())
}

pub fn write_document(path: &Path, bytes: &[u8]) -> io::Result<WriteReport> {
    write_report(path, bytes, false, |parent| {
        sync_file(&std::fs::File::open(parent)?)
    })
}

fn write_report(
    path: &Path,
    bytes: &[u8],
    private: bool,
    sync_parent: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<WriteReport> {
    let resolved;
    let path = if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        resolved = path.canonicalize()?;
        &resolved
    } else {
        path
    };
    let metadata = match std::fs::metadata(path) {
        Ok(m) => Some(m),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    if let Some(meta) = &metadata {
        if !meta.is_file() {
            return Err(io::Error::other("The save target is not a regular file."));
        }
        if meta.permissions().readonly() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "This file is read-only. Use Save a copy to choose another filename.",
            ));
        }
    }
    path.file_name()
        .ok_or_else(|| io::Error::other("No filename"))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = parent.join(format!(
        ".save-{}-{}-{}.tmp",
        std::process::id(),
        stamp,
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut created = false;
    let result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        created = true;
        if !private {
            if let Some(meta) = &metadata {
                file.set_permissions(meta.permissions())?;
            }
        }
        file.write_all(bytes)?;
        sync_file(&file)?;
        drop(file);
        if metadata.is_some() {
            std::fs::rename(&temp, path)?;
        } else {
            crate::workspace_files::rename_no_replace(&temp, path)?;
        }
        let mut report = WriteReport { warning: None };
        #[cfg(unix)]
        if let Err(error) = sync_parent(parent) {
            report.warning = Some(format!(
                "Saved, but directory durability could not be confirmed: {}",
                error
            ));
        }
        #[cfg(not(unix))]
        let _ = sync_parent;
        Ok(report)
    })();
    if result.is_err() && created {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
pub fn write_with_failed_directory_sync(path: &Path, bytes: &[u8]) -> io::Result<WriteReport> {
    write_report(path, bytes, false, |_| {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "test directory sync refusal",
        ))
    })
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
