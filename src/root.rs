use std::path::PathBuf;

/// Everything the app owns lives beside the executable.
///
/// Derived from the executable's own location, never from the working
/// directory and never from a constant, so the tree can be renamed or moved
/// without breaking.
pub fn app_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.canonicalize().ok())
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Human name of the app, taken from the one manifest field that declares it.
pub fn app_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}
