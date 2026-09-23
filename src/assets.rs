use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The folder the open document lives in. The page may read files from here
/// and nowhere else.
static SCOPE: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn set_scope(dir: Option<&Path>) {
    if let Ok(mut scope) = SCOPE.lock() {
        *scope = dir.map(PathBuf::from);
    }
}

fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(*byte as char)
            }
            b'\\' => out.push('/'),
            other => out.push_str(&format!("%{:02X}", other)),
        }
    }
    out
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Turns a reference in a document into an absolute path.
pub fn resolve(target: &str, base: Option<&Path>) -> String {
    let path = Path::new(target);
    if path.is_absolute() {
        return path.display().to_string();
    }
    match base {
        Some(dir) => dir.join(path).display().to_string(),
        None => path.display().to_string(),
    }
}

/// A URL the page can fetch through the app's own protocol. Windows serves
/// custom schemes over a localhost host name; the others use the scheme
/// directly.
pub fn url_for(target: &str, base: Option<&Path>) -> String {
    if target.starts_with("http://") || target.starts_with("https://")
        || target.starts_with("data:")
    {
        return target.to_string();
    }
    let absolute = resolve(target, base);
    let encoded = encode(&absolute);
    if cfg!(target_os = "windows") {
        format!("http://asset.localhost/{}", encoded.trim_start_matches('/'))
    } else {
        format!("asset://localhost/{}", encoded)
    }
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("avif") => "image/avif",
        _ => "application/octet-stream",
    }
}

/// Serves a file to the page, but only from inside the open document's folder.
pub fn serve(uri: &str) -> (Vec<u8>, &'static str, u16) {
    let path_part = uri
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(uri);
    let path_part = path_part.split_once('/').map(|(_, rest)| rest).unwrap_or("");
    let decoded = decode(path_part.split('?').next().unwrap_or(""));

    let mut candidate = PathBuf::from(&decoded);
    if !candidate.is_absolute() {
        candidate = PathBuf::from(format!("/{}", decoded));
    }

    let scope = SCOPE.lock().ok().and_then(|s| s.clone());
    let Some(scope) = scope else {
        return (b"no document open".to_vec(), "text/plain", 403);
    };
    let (Ok(real), Ok(scope_real)) = (candidate.canonicalize(), scope.canonicalize()) else {
        return (b"not found".to_vec(), "text/plain", 404);
    };
    if !real.starts_with(&scope_real) {
        // Outside the document's folder: refused rather than served.
        return (b"outside the document folder".to_vec(), "text/plain", 403);
    }
    match std::fs::read(&real) {
        Ok(bytes) => (bytes, mime_for(&real), 200),
        Err(_) => (b"not found".to_vec(), "text/plain", 404),
    }
}
