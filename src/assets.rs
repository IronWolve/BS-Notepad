use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The folder the open document lives in. The page may read files from here
/// and nowhere else.
static SCOPE: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn set_scope(dir: Option<&Path>) {
    if let Ok(mut scope) = SCOPE.lock() {
        *scope = dir.and_then(|path| path.canonicalize().ok());
    }
}

/// Normalize dot segments without asking the filesystem to resolve a path.
fn lexical(path: &Path) -> Option<PathBuf> {
    use std::path::Component;
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !matches!(result.components().next_back(), Some(Component::Normal(_))) {
                    return None;
                }
                result.pop();
            }
            _ => result.push(component.as_os_str()),
        }
    }
    Some(result)
}

pub fn scoped_image_path(path: &Path, scope: &Path) -> Option<PathBuf> {
    let path = lexical(path)?;
    let scope = lexical(scope)?;
    image_type(&path)?;
    crate::workspace_files::rebase(&path, &scope, &scope)
}

fn resolve_scoped_image(path: &Path, scope: &Path) -> Option<PathBuf> {
    let mut path = scoped_image_path(path, scope)?;
    for _ in 0..40 {
        let parts: Vec<_> = path.strip_prefix(scope).ok()?.components().collect();
        let mut current = scope.to_path_buf();
        let mut redirect = None;
        for (index, part) in parts.iter().enumerate() {
            current.push(part.as_os_str());
            let meta = std::fs::symlink_metadata(&current).ok()?;
            let linked = meta.file_type().is_symlink();
            #[cfg(target_os = "windows")]
            let linked = {
                use std::os::windows::fs::MetadataExt;
                linked
                    || meta.file_attributes() & 0x400 != 0
                        && !crate::file_metadata::cloud_placeholder(&current)
            };
            if linked {
                let target = std::fs::read_link(&current).ok()?;
                let mut target = if target.is_absolute() {
                    target
                } else {
                    current.parent()?.join(target)
                };
                for remainder in &parts[index + 1..] {
                    target.push(remainder.as_os_str());
                }
                // Validate the link target without resolving it or contacting another host.
                redirect = Some(scoped_image_path(&target, scope)?);
                break;
            }
            if index + 1 == parts.len() && !meta.is_file() {
                return None;
            }
        }
        if let Some(next) = redirect {
            path = next;
        } else {
            return Some(path);
        }
    }
    None
}

fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{:02X}", other)),
        }
    }
    out
}

pub fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let digit = |b: u8| (b as char).to_digit(16);
            if let (Some(a), Some(b)) = (digit(bytes[i + 1]), digit(bytes[i + 2])) {
                out.push((a * 16 + b) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
use crate::formatting::escape;
pub fn fragment(value: &str) -> String {
    let decoded = decode(value.trim_start_matches('#'));
    let decoded = decoded.strip_prefix("user-content-").unwrap_or(&decoded);
    if decoded.is_empty() {
        "doc-top".into()
    } else if decoded.starts_with("doc-heading-") || decoded.starts_with("doc-footnote-") {
        decoded.into()
    } else {
        format!("doc-heading-{}", decoded)
    }
}
pub fn local_target(value: &str, base: Option<&Path>) -> Option<(String, String)> {
    let value = value.trim();
    if value.chars().any(char::is_control) || value.starts_with("//") {
        return None;
    }
    if let Some((scheme, rest)) = value.split_once(':') {
        let uri_scheme = scheme
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
            && scheme
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b));
        let drive = scheme.len() == 1
            && scheme.as_bytes()[0].is_ascii_alphabetic()
            && (rest.starts_with('/') || rest.starts_with('\\'));
        if uri_scheme && !drive {
            return None;
        }
    }
    let (path, anchor) = value.split_once('#').unwrap_or((value, ""));
    let path = if path.starts_with(r"\\?\") {
        path
    } else {
        path.split('?').next().unwrap_or(path)
    };
    Some((
        resolve(&decode(path), base),
        if value.contains('#') {
            fragment(anchor)
        } else {
            String::new()
        },
    ))
}
pub fn external(value: &str) -> Option<String> {
    let value = value.trim();
    if value.chars().any(char::is_control) {
        return None;
    }
    let (scheme, rest) = value.split_once(':')?;
    let scheme = scheme.to_ascii_lowercase();
    matches!(scheme.as_str(), "http" | "https" | "mailto").then(|| format!("{}:{}", scheme, rest))
}
pub fn link_html(value: &str, title: &str, base: Option<&Path>) -> String {
    let attrs = if let Some(url) = external(value) {
        format!("href=\"{}\" data-external=\"1\"", escape(&url))
    } else if value.starts_with('#') {
        format!("href=\"#{}\"", escape(&fragment(value)))
    } else if let Some((path, anchor)) = local_target(value, base) {
        format!(
            "href=\"#\" data-open=\"{}\" data-fragment=\"{}\"",
            escape(&path),
            escape(&anchor)
        )
    } else {
        String::new()
    };
    format!("<a {} title=\"{}\">", attrs, escape(title))
}
pub fn image_url(value: &str, base: Option<&Path>, remote: bool) -> Option<String> {
    if let Some(url) = external(value) {
        return (remote && !url.starts_with("mailto:")).then_some(url);
    }
    if let Some(rest) = value.strip_prefix("data:") {
        return [
            "image/png;",
            "image/jpeg;",
            "image/gif;",
            "image/webp;",
            "image/avif;",
        ]
        .iter()
        .any(|p| rest.starts_with(p))
        .then(|| value.to_string());
    }
    let (path, _) = local_target(value, base)?;
    let path = scoped_image_path(Path::new(&path), base?)?;
    Some(url_for(&path.to_string_lossy(), None))
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
    if target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("data:")
    {
        return target.to_string();
    }
    asset_url(&resolve(target, base), cfg!(target_os = "windows"))
}
fn asset_url(path: &str, windows: bool) -> String {
    let normalized = if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!("//{}", rest.replace('\\', "/"))
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.replace('\\', "/")
    } else if windows {
        path.replace('\\', "/")
    } else {
        path.to_string()
    };
    let encoded = encode(&normalized);
    // Preserve both UNC separators; trimming them loses the server/share root.
    if windows {
        format!("http://asset.localhost/{}", encoded)
    } else {
        format!("asset://localhost/{}", encoded)
    }
}

pub fn brand_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "http://asset.localhost/__ui/logo.png"
    } else {
        "asset://localhost/__ui/logo.png"
    }
}

pub fn brand_banner_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "http://asset.localhost/__ui/banner.png"
    } else {
        "asset://localhost/__ui/banner.png"
    }
}

pub const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "jfif", "gif", "webp", "bmp", "ico", "svg", "avif",
];
pub const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;
pub fn image_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("PNG"),
        "jpg" | "jpeg" | "jfif" => Some("JPEG"),
        "gif" => Some("GIF"),
        "webp" => Some("WebP"),
        "bmp" => Some("BMP"),
        "ico" => Some("ICO"),
        "svg" => Some("SVG"),
        "avif" => Some("AVIF"),
        _ => None,
    }
}
fn mime_for(path: &Path) -> &'static str {
    match image_type(path) {
        Some("PNG") => "image/png",
        Some("JPEG") => "image/jpeg",
        Some("GIF") => "image/gif",
        Some("WebP") => "image/webp",
        Some("BMP") => "image/bmp",
        Some("ICO") => "image/x-icon",
        Some("SVG") => "image/svg+xml",
        Some("AVIF") => "image/avif",
        _ => "application/octet-stream",
    }
}

/// Serves a file to the page, but only from inside the open document's folder.
pub fn serve(uri: &str) -> (Vec<u8>, &'static str, u16) {
    let path_part = uri.split_once("://").map(|(_, rest)| rest).unwrap_or(uri);
    let path_part = path_part
        .split_once('/')
        .map(|(_, rest)| rest)
        .unwrap_or("");
    let decoded = decode(path_part.split('?').next().unwrap_or(""));
    if decoded.trim_start_matches('/') == "__ui/banner.png" {
        return (
            include_bytes!(concat!(env!("OUT_DIR"), "/help-banner.png")).to_vec(),
            "image/png",
            200,
        );
    }
    if decoded.trim_start_matches('/') == "__ui/logo.png" {
        return (
            include_bytes!("../pics/brand.png").to_vec(),
            "image/png",
            200,
        );
    }

    let mut candidate = PathBuf::from(&decoded);
    if !candidate.is_absolute() {
        candidate = PathBuf::from(format!("/{}", decoded));
    }

    let scope = SCOPE.lock().ok().and_then(|s| s.clone());
    let Some(scope) = scope else {
        return (b"no document open".to_vec(), "text/plain", 403);
    };
    // No canonicalize/stat/open on a document-supplied path until its lexical
    // location is authorized. Reject links/reparse points before following them.
    let Some(candidate) = scoped_image_path(&candidate, &scope) else {
        return (b"outside the image scope".to_vec(), "text/plain", 403);
    };
    let Some(candidate) = resolve_scoped_image(&candidate, &scope) else {
        return (
            b"image path unavailable or outside scope".to_vec(),
            "text/plain",
            403,
        );
    };
    let Ok(real) = candidate.canonicalize() else {
        return (b"not found".to_vec(), "text/plain", 404);
    };
    if !real.starts_with(&scope) {
        // Outside the document's folder: refused rather than served.
        return (b"outside the document folder".to_vec(), "text/plain", 403);
    }
    use std::io::Read;
    const LIMIT: u64 = MAX_IMAGE_BYTES;
    match std::fs::File::open(&real) {
        Ok(file) => {
            if file
                .metadata()
                .map(|m| !m.is_file() || m.len() > LIMIT)
                .unwrap_or(true)
            {
                return (b"asset exceeds size limit".to_vec(), "text/plain", 413);
            }
            let mut bytes = Vec::new();
            if file.take(LIMIT + 1).read_to_end(&mut bytes).is_err() {
                return (b"cannot read asset".to_vec(), "text/plain", 404);
            }
            if bytes.len() as u64 > LIMIT {
                return (b"asset exceeds size limit".to_vec(), "text/plain", 413);
            }
            (bytes, mime_for(&real), 200)
        }
        Err(_) => (b"not found".to_vec(), "text/plain", 404),
    }
}

/// Two workers bound file-read memory while queued requests hold only a URI and responder.
pub fn worker_pool() -> std::sync::mpsc::Sender<(String, wry::RequestAsyncResponder)> {
    let (sender, receiver) = std::sync::mpsc::channel::<(String, wry::RequestAsyncResponder)>();
    let receiver = std::sync::Arc::new(Mutex::new(receiver));
    for _ in 0..2 {
        let receiver = receiver.clone();
        std::thread::spawn(move || loop {
            let request = receiver.lock().unwrap().recv();
            let Ok((uri, responder)) = request else {
                break;
            };
            let (body, mime, status) = serve(&uri);
            responder.respond(
                wry::http::Response::builder()
                    .status(status)
                    .header("Content-Type", mime)
                    .header("X-Content-Type-Options", "nosniff")
                    .body(body)
                    .unwrap(),
            );
        });
    }
    sender
}

#[cfg(test)]
mod path_tests {
    use super::*;
    #[test]
    fn native_windows_image_paths_survive_url_encoding() {
        let drive = asset_url(r"\\?\C:\Pictures\a #1.png", true);
        assert_eq!(drive, "http://asset.localhost/C%3A/Pictures/a%20%231.png");
        let unc = asset_url(r"\\?\UNC\server\share\photo.png", true);
        assert_eq!(unc, "http://asset.localhost///server/share/photo.png");
        assert_eq!(
            decode(unc.strip_prefix("http://asset.localhost/").unwrap()),
            "//server/share/photo.png"
        );
    }
}
