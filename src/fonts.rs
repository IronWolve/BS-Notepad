use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct FontFamily {
    pub name: String,
    pub monospace: bool,
    /// Patched families carry extra glyphs and are usually what is wanted for
    /// code, so they are worth singling out in the picker.
    pub nerd: bool,
}

/// The page cannot list installed fonts - the API for that exists in one engine
/// only and is permission gated - so the list is built here and handed over.
pub fn families() -> Vec<FontFamily> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut seen: std::collections::BTreeMap<String, (bool, bool)> = Default::default();
    for face in db.faces() {
        let Some((name, _)) = face.families.first() else {
            continue;
        };
        let lower = name.to_lowercase();
        let nerd =
            lower.contains("nerd font") || lower.ends_with(" nf") || lower.contains("nerdfont");
        let entry = seen.entry(name.clone()).or_insert((face.monospaced, nerd));
        entry.0 |= face.monospaced;
        entry.1 |= nerd;
    }

    seen.into_iter()
        .map(|(name, (monospace, nerd))| FontFamily {
            name,
            monospace,
            nerd,
        })
        .collect()
}

/// True when the family is actually present, so a setting carried to another
/// machine can be reported as missing instead of silently rendering something
/// else.
pub fn has_family(list: &[FontFamily], wanted: &str) -> bool {
    let first = wanted
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('"');
    if first.is_empty() {
        return true;
    }
    let generic = [
        "system-ui",
        "-apple-system",
        "ui-monospace",
        "sans-serif",
        "serif",
        "monospace",
        "cursive",
    ];
    if generic.contains(&first.to_lowercase().as_str()) {
        return true;
    }
    list.iter().any(|f| f.name.eq_ignore_ascii_case(first))
}
