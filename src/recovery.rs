use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Clone, Serialize, Deserialize)]
pub struct Draft {
    pub path: Option<PathBuf>,
    pub source: String,
    pub saved_source: String,
    pub format: crate::storage::TextFormat,
    pub fingerprint: Option<u64>,
}
pub fn key() -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}
pub fn write(root: &Path, key: &str, draft: Option<&Draft>) -> std::io::Result<()> {
    if key.is_empty() || !key.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        return Err(std::io::Error::other("Invalid recovery key"));
    }
    let dir = root.join("recovery");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", key));
    if let Some(draft) = draft {
        crate::storage::write_atomic(&path, &serde_json::to_vec(draft)?)
    } else {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}
pub fn read(root: &Path) -> Vec<(String, Draft)> {
    let mut drafts = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root.join("recovery")) {
        for entry in entries.flatten().take(100) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if entry
                .metadata()
                .map(|m| m.len() > 128 * 1024 * 1024)
                .unwrap_or(true)
            {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(draft) = serde_json::from_slice(&bytes) {
                    drafts.push((
                        path.file_stem().unwrap().to_string_lossy().into_owned(),
                        draft,
                    ));
                }
            }
        }
    }
    drafts
}
