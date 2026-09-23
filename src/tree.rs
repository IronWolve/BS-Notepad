use std::path::Path;

use serde::Serialize;

#[derive(Serialize)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub dir: bool,
    pub openable: bool,
}

const TEXTUAL: &[&str] = &[
    "md", "markdown", "mdown", "mkd", "mkdn", "mdx", "txt", "text", "rst", "org",
    "rs", "py", "js", "ts", "jsx", "tsx", "go", "c", "h", "cpp", "hpp", "cs", "java",
    "rb", "php", "sh", "bash", "zsh", "ps1", "lua", "sql", "toml", "yaml", "yml",
    "json", "xml", "html", "css", "scss", "ini", "conf", "cfg", "log", "csv",
];

fn openable(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => TEXTUAL.contains(&ext.to_lowercase().as_str()),
        None => false,
    }
}

/// One directory level. Folders open on demand rather than the whole tree
/// being walked up front.
pub fn list(dir: &Path) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let Ok(read) = std::fs::read_dir(dir) else { return entries };

    for item in read.flatten() {
        let path = item.path();
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let dir = path.is_dir();
        entries.push(Entry {
            name,
            path: path.display().to_string(),
            dir,
            openable: !dir && openable(&path),
        });
    }

    entries.sort_by(|a, b| match (a.dir, b.dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    entries
}
