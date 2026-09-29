use std::path::Path;

use serde::Serialize;

#[derive(Serialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub dir: bool,
    pub openable: bool,
    pub modified: Option<u64>,
}

const TEXTUAL: &[&str] = &[
    "md", "markdown", "mdown", "mkd", "mkdn", "mdx", "txt", "text", "rst", "org", "rs", "py", "js",
    "ts", "jsx", "tsx", "go", "c", "h", "cpp", "hpp", "cs", "java", "rb", "php", "sh", "bash",
    "zsh", "ps1", "lua", "sql", "toml", "yaml", "yml", "json", "xml", "html", "css", "scss", "ini",
    "conf", "cfg", "log", "csv",
];

pub(crate) fn openable(path: &Path) -> bool {
    if crate::assets::image_type(path).is_some() {
        return true;
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => TEXTUAL.contains(&ext.to_lowercase().as_str()),
        None => true,
    }
}

/// One directory level. Folders open on demand rather than the whole tree
/// being walked up front.
#[cfg(test)]
pub fn list(dir: &Path, show_hidden: bool) -> std::io::Result<Vec<Entry>> {
    list_with_options(dir, show_hidden, false, false)
}
pub fn list_with_options(
    dir: &Path,
    show_hidden: bool,
    dates: bool,
    sort_date: bool,
) -> std::io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for item in std::fs::read_dir(dir)? {
        let item = item?;
        let path = item.path();
        let name = item.file_name().to_string_lossy().into_owned();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        let kind = item.file_type()?;
        let metadata = if dates || sort_date || kind.is_symlink() || cfg!(target_os = "windows") {
            std::fs::metadata(&path).ok()
        } else {
            None
        };
        #[cfg(target_os = "windows")]
        if !show_hidden {
            use std::os::windows::fs::MetadataExt;
            if metadata
                .as_ref()
                .is_some_and(|m| m.file_attributes() & 6 != 0)
            {
                continue;
            }
        }
        let directory =
            kind.is_dir() || kind.is_symlink() && metadata.as_ref().is_some_and(|m| m.is_dir());
        let regular =
            kind.is_file() || kind.is_symlink() && metadata.as_ref().is_some_and(|m| m.is_file());
        let modified = metadata
            .and_then(|m| m.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|time| time.as_millis().min(u64::MAX as u128) as u64);
        entries.push(Entry {
            name,
            path: path.to_string_lossy().into_owned(),
            dir: directory,
            openable: regular && path.to_str().is_some() && openable(&path),
            modified,
        });
        if entries.len() > 10_000 {
            return Err(std::io::Error::other("This folder contains more than 10,000 visible entries. Use Quick Open to find a file, or open a smaller subfolder."));
        }
    }
    entries.sort_by_cached_key(|entry| {
        (
            !entry.dir,
            std::cmp::Reverse(if sort_date {
                entry.modified.unwrap_or(0)
            } else {
                0
            }),
            entry.name.to_lowercase(),
            entry.name.clone(),
        )
    });
    Ok(entries)
}
