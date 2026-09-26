use serde::Serialize;

/// A theme colours the document chrome and the code together, so the page and
/// its code blocks always agree.
#[derive(Serialize, Clone)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub dark: bool,
    pub bg: String,
    pub fg: String,
    pub panel: String,
    pub bar: String,
    pub rule: String,
    pub link: String,
    pub dim: String,
    pub accent: String,
    /// Which colouring set the code blocks use.
    #[serde(skip)]
    pub code: String,
    #[serde(skip)]
    pub text_contrast: u32,
}

#[allow(clippy::too_many_arguments)]
fn t(
    id: &str,
    name: &str,
    dark: bool,
    bg: &str,
    fg: &str,
    panel: &str,
    bar: &str,
    rule: &str,
    link: &str,
    dim: &str,
    accent: &str,
    code: &str,
) -> Theme {
    Theme {
        id: id.into(),
        name: name.into(),
        dark,
        bg: bg.into(),
        fg: fg.into(),
        panel: panel.into(),
        bar: bar.into(),
        rule: rule.into(),
        link: link.into(),
        dim: dim.into(),
        accent: accent.into(),
        code: code.into(),
        text_contrast: 0,
    }
}

pub fn builtin() -> &'static [Theme] {
    static THEMES: std::sync::OnceLock<Vec<Theme>> = std::sync::OnceLock::new();
    THEMES.get_or_init(|| {
        vec![
            t(
                "bold-crimson",
                "Crimson",
                true,
                "#A03C3C",
                "#fff7f0",
                "#933737",
                "#A03C3C",
                "#b15e5c",
                "#fff7f0",
                "#f1dbd5",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-tangerine",
                "Tangerine",
                false,
                "#C85F3C",
                "#101416",
                "#cc6c4c",
                "#C85F3C",
                "#a85337",
                "#101416",
                "#322726",
                "#101416",
                "Github",
            ),
            t(
                "bold-sunflower",
                "Sunflower",
                false,
                "#FFDE30",
                "#101416",
                "#ffe141",
                "#FFDE30",
                "#d5bb2d",
                "#101416",
                "#3a3a24",
                "#101416",
                "Github",
            ),
            t(
                "bold-emerald",
                "Emerald",
                false,
                "#388E70",
                "#101416",
                "#48977b",
                "#388E70",
                "#327a62",
                "#101416",
                "#1c2e2e",
                "#101416",
                "Github",
            ),
            t(
                "bold-turquoise",
                "Turquoise",
                false,
                "#58C9B9",
                "#101416",
                "#65cdbf",
                "#58C9B9",
                "#4caa9e",
                "#101416",
                "#213739",
                "#101416",
                "Github",
            ),
            t(
                "bold-cobalt",
                "Cobalt",
                true,
                "#1E3F66",
                "#fff7f0",
                "#1c3a5e",
                "#1E3F66",
                "#46607f",
                "#fff7f0",
                "#dddbdb",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-violet",
                "Violet",
                true,
                "#8C5D91",
                "#fff7f0",
                "#815685",
                "#8C5D91",
                "#a179a2",
                "#fff7f0",
                "#eee0e2",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-pink",
                "Pink",
                false,
                "#E4707C",
                "#101416",
                "#e67b86",
                "#E4707C",
                "#bf616c",
                "#101416",
                "#362930",
                "#101416",
                "Github",
            ),
            t(
                "bold-burgundy",
                "Burgundy",
                true,
                "#4A1C1C",
                "#fff7f0",
                "#441a1a",
                "#4A1C1C",
                "#6b4342",
                "#fff7f0",
                "#e4d6d0",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-mulberry",
                "Mulberry",
                true,
                "#8C4356",
                "#fff7f0",
                "#813e4f",
                "#8C4356",
                "#a16372",
                "#fff7f0",
                "#eedcd9",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-plum",
                "Plum",
                true,
                "#5B445F",
                "#fff7f0",
                "#543f57",
                "#5B445F",
                "#796479",
                "#fff7f0",
                "#e6dcda",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-forest",
                "Forest",
                true,
                "#163229",
                "#fff7f0",
                "#142e26",
                "#163229",
                "#40554d",
                "#fff7f0",
                "#dcd9d2",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-deep-teal",
                "Deep Teal",
                true,
                "#2A5254",
                "#fff7f0",
                "#274b4d",
                "#2A5254",
                "#507070",
                "#fff7f0",
                "#dfded9",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-amber",
                "Amber",
                true,
                "#A3682C",
                "#ffffff",
                "#966028",
                "#A3682C",
                "#b48352",
                "#ffffff",
                "#f1e8df",
                "#ffffff",
                "Base16OceanDark",
            ),
            t(
                "bold-chestnut",
                "Chestnut",
                true,
                "#5D4037",
                "#fff7f0",
                "#563b33",
                "#5D4037",
                "#7a6158",
                "#fff7f0",
                "#e7dcd4",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-copper",
                "Copper",
                true,
                "#7D4C3D",
                "#fff7f0",
                "#734638",
                "#7D4C3D",
                "#946b5d",
                "#fff7f0",
                "#ecddd5",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-midnight-blue",
                "Midnight Blue",
                true,
                "#102349",
                "#fff7f0",
                "#0f2043",
                "#102349",
                "#3b4967",
                "#fff7f0",
                "#dbd7d7",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-azure",
                "Azure",
                false,
                "#3D7BAD",
                "#000000",
                "#4d86b4",
                "#3D7BAD",
                "#32658e",
                "#000000",
                "#09121a",
                "#000000",
                "Github",
            ),
            t(
                "bold-sky-blue",
                "Sky Blue",
                false,
                "#689AAF",
                "#101416",
                "#74a2b5",
                "#689AAF",
                "#588293",
                "#101416",
                "#1d282d",
                "#101416",
                "Github",
            ),
            t(
                "bold-denim",
                "Denim",
                true,
                "#4c6486",
                "#fff7f0",
                "#465c7b",
                "#4c6486",
                "#6c7e99",
                "#fff7f0",
                "#e4e1e0",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-olive",
                "Olive",
                true,
                "#615020",
                "#fff7f0",
                "#594a1d",
                "#615020",
                "#7d6e45",
                "#fff7f0",
                "#e7ded1",
                "#fff7f0",
                "Base16OceanDark",
            ),
            t(
                "bold-gold",
                "Gold",
                false,
                "#C6A75C",
                "#101416",
                "#cbae69",
                "#C6A75C",
                "#a58d4f",
                "#101416",
                "#2b2a20",
                "#101416",
                "Github",
            ),
            t(
                "bold-black",
                "Black",
                true,
                "#000000",
                "#dcdcdc",
                "#080808",
                "#000000",
                "#242424",
                "#ededed",
                "#a0a0a0",
                "#dcdcdc",
                "Base16OceanDark",
            ),
            t(
                "rose-stone",
                "Rose Stone",
                false,
                "#FFF7F7",
                "#4A1C1C",
                "#FFE4E4",
                "#FFF7F7",
                "#FFD4D4",
                "#A03C3C",
                "#4A1C1C",
                "#A03C3C",
                "Github",
            ),
            t(
                "warm-clay",
                "Warm Clay",
                false,
                "#F8E3C4",
                "#2C1810",
                "#E6C89B",
                "#F8E3C4",
                "#D4B176",
                "#C85F3C",
                "#2C1810",
                "#C85F3C",
                "Github",
            ),
            t(
                "cocoa", "Cocoa", false, "#EFEBE9", "#5D4037", "#D7CCC8", "#EFEBE9", "#BCAAA4",
                "#795548", "#5D4037", "#795548", "Github",
            ),
            t(
                "parchment",
                "Parchment",
                false,
                "#F4E4BE",
                "#5F4B32",
                "#E8D5AB",
                "#F4E4BE",
                "#E2CCA0",
                "#A3682C",
                "#5F4B32",
                "#A3682C",
                "Github",
            ),
            t(
                "jade", "Jade", false, "#E8F4EE", "#1D4A3C", "#DCEEE5", "#E8F4EE", "#D5EAE0",
                "#388E70", "#1D4A3C", "#388E70", "Github",
            ),
            t(
                "lagoon", "Lagoon", false, "#E6F3F4", "#2A5254", "#D4EBEC", "#E6F3F4", "#C2E3E4",
                "#3C707A", "#2A5254", "#3C707A", "Github",
            ),
            t(
                "porcelain",
                "Porcelain",
                false,
                "#F2F7FF",
                "#102349",
                "#D4E5FF",
                "#F2F7FF",
                "#B9D7FF",
                "#1E3F66",
                "#102349",
                "#1E3F66",
                "Github",
            ),
            t(
                "lavender-clay",
                "Lavender Clay",
                false,
                "#F4EEF5",
                "#442A47",
                "#E8DCEA",
                "#F4EEF5",
                "#DBCBDE",
                "#8C5D91",
                "#442A47",
                "#8C5D91",
                "Github",
            ),
            t(
                "blossom", "Blossom", false, "#FFEEF2", "#8C4356", "#FFE6EC", "#FFEEF2", "#FFDEE6",
                "#E4707C", "#8C4356", "#E4707C", "Github",
            ),
            t(
                "pewter", "Pewter", false, "#F3F4F6", "#1F2937", "#E5E7EB", "#F3F4F6", "#D1D5DB",
                "#4B5563", "#1F2937", "#4B5563", "Github",
            ),
            t(
                "ink",
                "Ink",
                true,
                "#1F2937",
                "#E5E7EB",
                "#4B5563",
                "#1F2937",
                "#374151",
                "#4B5563",
                "#E5E7EB",
                "#4B5563",
                "Base16OceanDark",
            ),
            t(
                "marble", "Marble", false, "#F7F6F2", "#2B2926", "#F2F0EB", "#F7F6F2", "#EAE8E3",
                "#9A958E", "#2B2926", "#9A958E", "Github",
            ),
            t(
                "light", "Light", false, "#ffffff", "#24292f", "#f6f8fa", "#f0f3f6", "#d0d7de",
                "#0969da", "#6e7781", "#0969da", "Github",
            ),
            t(
                "mist", "Mist", false, "#c8cfd7", "#283541", "#b9c3ce", "#adb9c6", "#8c9aaa",
                "#215581", "#3b4b5b", "#276694", "Github",
            ),
            t(
                "sage", "Sage", false, "#becbc3", "#293b33", "#afc0b5", "#a1b7a9", "#869d8f",
                "#285d46", "#354d3e", "#2b6c50", "Github",
            ),
            t(
                "slate", "Slate", true, "#536273", "#f2f5f8", "#475768", "#3e4e60", "#748699",
                "#c0e5ff", "#d1dce7", "#9dd7ff", "Nord",
            ),
            t(
                "graphite",
                "Graphite",
                true,
                "#57595d",
                "#f5f5f5",
                "#4a4c50",
                "#414347",
                "#7a7d82",
                "#c4dff5",
                "#d5d7db",
                "#acd2f0",
                "Base16OceanDark",
            ),
            t(
                "dark",
                "Dark",
                true,
                "#1f1f1f",
                "#d7d7d7",
                "#181818",
                "#252526",
                "#343434",
                "#75beff",
                "#a5a5a5",
                "#3794ff",
                "Base16OceanDark",
            ),
            t(
                "dracula", "Dracula", true, "#282a36", "#f8f8f2", "#21222c", "#191a21", "#44475a",
                "#bd93f9", "#6272a4", "#ff79c6", "Dracula",
            ),
            t(
                "solarized-dark",
                "Solarized Dark",
                true,
                "#002b36",
                "#93a1a1",
                "#073642",
                "#00212b",
                "#094959",
                "#268bd2",
                "#586e75",
                "#b58900",
                "SolarizedDark",
            ),
            t(
                "solarized-light",
                "Solarized Light",
                false,
                "#fdf6e3",
                "#586e75",
                "#eee8d5",
                "#e8e1cc",
                "#d7cfb8",
                "#268bd2",
                "#93a1a1",
                "#b58900",
                "SolarizedLight",
            ),
            t(
                "nord", "Nord", true, "#2e3440", "#d8dee9", "#3b4252", "#2b3240", "#4c566a",
                "#88c0d0", "#7b88a1", "#81a1c1", "Nord",
            ),
            t(
                "gruvbox",
                "Gruvbox Dark",
                true,
                "#282828",
                "#ebdbb2",
                "#32302f",
                "#1d2021",
                "#504945",
                "#83a598",
                "#928374",
                "#fabd2f",
                "GruvboxDark",
            ),
            t(
                "monokai",
                "Monokai",
                true,
                "#272822",
                "#f8f8f2",
                "#2f302a",
                "#1e1f1a",
                "#49483e",
                "#66d9ef",
                "#75715e",
                "#a6e22e",
                "MonokaiExtended",
            ),
        ]
    })
}

pub fn find(id: &str) -> Theme {
    builtin()
        .iter()
        .find(|t| t.id == id)
        .unwrap_or_else(|| builtin().iter().find(|t| t.id == "dark").unwrap())
        .clone()
}

/// Each theme names the colouring set its code blocks use, so the page and the
/// code always come from the same palette rather than an approximation.
pub fn code_theme_name(id: &str) -> two_face::theme::EmbeddedThemeName {
    use two_face::theme::EmbeddedThemeName as E;
    match find(id).code.as_str() {
        "Github" => E::Github,
        "Dracula" => E::Dracula,
        "SolarizedDark" => E::SolarizedDark,
        "SolarizedLight" => E::SolarizedLight,
        "Nord" => E::Nord,
        "GruvboxDark" => E::GruvboxDark,
        "MonokaiExtended" => E::MonokaiExtended,
        _ => E::Base16OceanDark,
    }
}

fn channel(hex: &str, at: usize) -> f64 {
    u8::from_str_radix(hex.get(at..at + 2).unwrap_or("00"), 16).unwrap_or(0) as f64 / 255.0
}

fn luminance(hex: &str) -> f64 {
    let h = hex.trim_start_matches('#');
    let f = |c: f64| {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(channel(h, 0)) + 0.7152 * f(channel(h, 2)) + 0.0722 * f(channel(h, 4))
}

/// Contrast ratio between two colours, the usual accessibility measure.
pub fn contrast(a: &str, b: &str) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    let (hi, lo) = if x > y { (x, y) } else { (y, x) };
    (hi + 0.05) / (lo + 0.05)
}

/// Lightens or darkens text until it is readable on its background, so no
/// combination of theme and font can end up unreadable.
pub fn guard(fg: &str, bg: &str, target: f64) -> String {
    if contrast(fg, bg) >= target {
        return fg.to_string();
    }
    let h = fg.trim_start_matches('#');
    let toward_light = contrast("#ffffff", bg) >= contrast("#000000", bg);
    let mut rgb: Vec<f64> = (0..3).map(|i| channel(h, i * 2) * 255.0).collect();
    for _ in 0..40 {
        for c in rgb.iter_mut() {
            *c = if toward_light {
                (*c + 8.0).min(255.0)
            } else {
                (*c - 8.0).max(0.0)
            };
        }
        let candidate = format!(
            "#{:02x}{:02x}{:02x}",
            rgb[0] as u8, rgb[1] as u8, rgb[2] as u8
        );
        if contrast(&candidate, bg) >= target {
            return candidate;
        }
    }
    if toward_light {
        "#ffffff".into()
    } else {
        "#000000".into()
    }
}

/// Increase text strength without altering the surface or quiet controls.
pub fn strengthen(fg: &str, bg: &str, amount: u32) -> String {
    if amount == 0 {
        return fg.into();
    }
    let end = if contrast("#ffffff", bg) >= contrast("#000000", bg) {
        255.0
    } else {
        0.0
    };
    let h = fg.trim_start_matches('#');
    let mix = amount.min(100) as f64 / 100.0;
    let rgb: Vec<u8> = (0..3)
        .map(|i| {
            let c = channel(h, i * 2) * 255.0;
            (c + (end - c) * mix).round() as u8
        })
        .collect();
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contrast_control_preserves_default_and_increases_readability() {
        for t in builtin() {
            let fg = guard(&t.fg, &t.bg, 4.5);
            assert_eq!(strengthen(&fg, &t.bg, 0), fg);
            let mut previous = contrast(&fg, &t.bg);
            for amount in [25, 50, 75, 100] {
                let current = contrast(&strengthen(&fg, &t.bg, amount), &t.bg);
                assert!(current >= previous, "{} at {}", t.id, amount);
                previous = current;
            }
        }
    }
    #[test]
    fn palette_ids_are_unique_and_survive_settings_normalization() {
        let themes = builtin();
        let ids: std::collections::HashSet<_> = themes.iter().map(|t| &t.id).collect();
        assert_eq!(ids.len(), themes.len());
        for t in themes {
            let mut settings = crate::settings::Settings {
                theme: t.id.clone(),
                text_contrast: 200,
                ..Default::default()
            };
            settings.normalize();
            assert_eq!(settings.theme, t.id);
            assert_eq!(settings.text_contrast, 100);
        }
    }
}
