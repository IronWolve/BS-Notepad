use std::{
    path::Path,
    time::{Duration, SystemTime},
};
/// Only dated app logs and explicitly marked release archives are eligible.
pub fn cleanup(root: &Path, days: u32, keep: u32) -> std::io::Result<(usize, usize)> {
    let mut removed = 0;
    let mut failed = 0;
    if let Ok(entries) = std::fs::read_dir(root.join("logs")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let stem = name.strip_suffix(".log").unwrap_or("");
            let dated = stem.len() == 10
                && stem.bytes().enumerate().all(|(i, b)| {
                    if i == 4 || i == 7 {
                        b == b'-'
                    } else {
                        b.is_ascii_digit()
                    }
                });
            if !dated || entry.file_type()?.is_symlink() || !entry.file_type()?.is_file() {
                continue;
            }
            if entry
                .metadata()?
                .modified()
                .ok()
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .is_some_and(|age| age > Duration::from_secs(days as u64 * 86400))
            {
                if std::fs::remove_file(entry.path()).is_ok() {
                    removed += 1;
                } else {
                    failed += 1;
                }
            }
        }
    }
    let mut archives = Vec::new();
    for entry in std::fs::read_dir(root)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("previous-")
            || !name[9..].bytes().all(|b| b.is_ascii_digit() || b == b'-')
            || !entry.file_type()?.is_dir()
            || entry.file_type()?.is_symlink()
        {
            continue;
        }
        if std::fs::read_to_string(entry.path().join(".release-backup"))
            .ok()
            .as_deref()
            == Some(env!("CARGO_PKG_NAME"))
        {
            archives.push(entry.path());
        }
    }
    archives.sort();
    archives.reverse();
    for archive in archives.into_iter().skip(keep.max(1) as usize) {
        let app = format!("{}.exe", env!("CARGO_PKG_NAME"));
        let held = format!("in-use-{}", app);
        let allowed = [
            app.as_str(),
            held.as_str(),
            "notepad.exe",
            "in-use-notepad.exe",
            "WebView2Loader.dll",
            "installed.json",
            "register-file-types.ps1",
            ".release-backup",
        ];
        let entries: Vec<_> = std::fs::read_dir(&archive)?.collect::<Result<_, _>>()?;
        if entries.iter().any(|e| {
            !allowed.contains(&e.file_name().to_string_lossy().as_ref())
                || !e
                    .file_type()
                    .map(|t| t.is_file() && !t.is_symlink())
                    .unwrap_or(false)
        }) {
            continue;
        }
        let mut all = true;
        for entry in entries
            .iter()
            .filter(|e| e.file_name() != ".release-backup")
        {
            if std::fs::remove_file(entry.path()).is_err() {
                all = false;
                failed += 1;
            }
        }
        if all {
            std::fs::remove_file(archive.join(".release-backup"))?;
            std::fs::remove_dir(archive)?;
            removed += 1;
        }
    }
    Ok((removed, failed))
}
