use std::path::PathBuf;

/// Everything the app owns lives beside the executable or its app bundle.
///
/// Derived from the executable's own location, never from the working
/// directory and never from a constant, so the tree can be renamed or moved
/// without breaking.
pub fn app_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.canonicalize().ok())
        .map(|exe| data_root(&exe))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Bundles keep mutable state beside the app so running it never modifies its code signature.
fn data_root(exe: &std::path::Path) -> PathBuf {
    let parent = exe.parent().unwrap_or_else(|| std::path::Path::new("."));
    if parent.file_name().is_some_and(|name| name == "MacOS") {
        if let Some(contents) = parent
            .parent()
            .filter(|path| path.file_name().is_some_and(|name| name == "Contents"))
        {
            if let Some(bundle) = contents
                .parent()
                .filter(|path| path.extension().is_some_and(|ext| ext == "app"))
            {
                return bundle
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(format!("{}-data", env!("CARGO_PKG_NAME")));
            }
        }
    }
    parent.to_path_buf()
}

/// Human name of the app, taken from the one manifest field that declares it.
pub fn app_name() -> &'static str {
    env!("APP_DISPLAY_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundle_data_is_outside_the_signed_app() {
        let root = std::path::Path::new("portable");
        assert_eq!(
            data_root(&root.join("Editor.app/Contents/MacOS/editor")),
            root.join(format!("{}-data", env!("CARGO_PKG_NAME")))
        );
        assert_eq!(data_root(&root.join("editor")), root);
    }
}

/// Test hooks require a private build and an explicit token in its disposable data root.
pub fn smoke_authorized(root: &std::path::Path) -> bool {
    #[cfg(feature = "smoke")]
    {
        let Ok(token) = std::env::var("SMOKE_TEST_TOKEN") else {
            return false;
        };
        token.len() >= 32
            && token.bytes().all(|b| b.is_ascii_hexdigit())
            && std::fs::read_to_string(root.join(".smoke-token"))
                .ok()
                .as_deref()
                == Some(token.as_str())
    }
    #[cfg(not(feature = "smoke"))]
    {
        let _ = root;
        false
    }
}
