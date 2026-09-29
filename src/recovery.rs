use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum Choice {
    Restore,
    Discard,
    Later,
}
pub fn choice(result: crate::dialogs::MessageDialogResult) -> Choice {
    match result {
        crate::dialogs::MessageDialogResult::Custom(label) if label == "Restore" => Choice::Restore,
        crate::dialogs::MessageDialogResult::Custom(label) if label == "Discard" => Choice::Discard,
        _ => Choice::Later,
    }
}
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
        crate::storage::write_private_atomic(&path, &serde_json::to_vec(draft)?)
    } else {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}
#[derive(Default)]
pub struct Scan {
    pub drafts: Vec<(String, Draft)>,
    pub skipped: usize,
}

pub fn scan(root: &Path) -> Scan {
    let mut result = Scan::default();
    let entries = match std::fs::read_dir(root.join("recovery")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return result,
        Err(error) => {
            crate::log::line(&format!("Recovery folder could not be read: {error}"));
            result.skipped = 1;
            return result;
        }
    };
    let mut candidates = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            result.skipped += 1;
            continue;
        };
        let path = entry.path();
        let key = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.extension().is_none_or(|extension| extension != "json")
            || key.is_empty()
            || !key.bytes().all(|b| b.is_ascii_digit() || b == b'-')
        {
            continue;
        }
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            result.skipped += 1;
            continue;
        }
        match entry.metadata() {
            Ok(meta) if meta.len() <= (crate::storage::MAX_EDIT_BYTES as u64 * 12 + 65536) => {
                candidates.push((meta.modified().unwrap_or(std::time::UNIX_EPOCH), path))
            }
            _ => result.skipped += 1,
        }
    }
    candidates.sort();
    let mut held = 0;
    for (_, path) in candidates {
        if result.drafts.len() >= 1000 || held >= 128 * 1024 * 1024 {
            result.skipped += 1;
            continue;
        }
        let draft = std::fs::File::open(&path).ok().and_then(|file| {
            serde_json::from_reader::<_, Draft>(std::io::BufReader::new(file)).ok()
        });
        let Some(draft) = draft else {
            result.skipped += 1;
            continue;
        };
        if !["UTF-8", "UTF-8 BOM", "UTF-16 LE", "UTF-16 BE"]
            .contains(&draft.format.encoding.as_str())
            || !["LF", "CRLF", "CR"].contains(&draft.format.ending.as_str())
        {
            result.skipped += 1;
            continue;
        }
        let bytes = draft.source.len() + draft.saved_source.len();
        if draft.source.len() > crate::storage::MAX_EDIT_BYTES
            || draft.saved_source.len() > crate::storage::MAX_EDIT_BYTES
            || held + bytes > 128 * 1024 * 1024
        {
            result.skipped += 1;
            continue;
        }
        held += bytes;
        result.drafts.push((
            path.file_stem().unwrap().to_string_lossy().into_owned(),
            draft,
        ));
    }
    result
}
#[cfg(any(test, feature = "smoke"))]
pub fn read(root: &Path) -> Vec<(String, Draft)> {
    scan(root).drafts
}
