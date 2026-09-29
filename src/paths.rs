use std::path::{Path, PathBuf};

/// Keep one display/identity form without folding case on case-sensitive volumes.
pub fn display_form(path: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    if let Some(text) = path.to_str() {
        if let Some(tail) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{tail}"));
        }
        if let Some(tail) = text.strip_prefix(r"\\?\") {
            if tail.as_bytes().get(1) == Some(&b':') {
                return PathBuf::from(tail);
            }
        }
    }
    path.to_owned()
}

pub fn absolute(path: &Path) -> PathBuf {
    display_form(&std::path::absolute(path).unwrap_or_else(|_| path.to_owned()))
}

/// Called on workers, or for an explicit Save. New files resolve their parent too.
pub fn normalize(path: &Path) -> PathBuf {
    if let Ok(real) = path.canonicalize() {
        return display_form(&real);
    }
    let path = absolute(path);
    if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
        if let Ok(parent) = parent.canonicalize() {
            return display_form(&parent.join(name));
        }
    }
    path
}

pub fn same(a: &Path, b: &Path) -> bool {
    display_form(a) == display_form(b)
}

/// Pure lexical check: no filesystem or network access is needed to ask first.
pub fn network_or_device(path: &Path) -> bool {
    let text = path.to_string_lossy();
    if cfg!(target_os = "windows") {
        let text = text.replace('/', "\\");
        text.starts_with("\\\\")
    } else {
        text == "/net" || text.starts_with("/net/") || text.starts_with("//")
    }
}

pub fn device(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        matches!(path.components().next(), Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::DeviceNS(_) | std::path::Prefix::Verbatim(_)))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        false
    }
}
