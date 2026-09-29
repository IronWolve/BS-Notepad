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
    pub mark_fg: String,
    pub input_rule: String,
    pub selected: String,
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

macro_rules! palette {
    ($($field:ident : $value:expr),* $(,)?) => {
        Theme { $($field: $value.into(),)* mark_fg: String::new(), input_rule: String::new(), selected: String::new(), text_contrast: 0 }
    };
}

pub fn builtin() -> &'static [Theme] {
    static THEMES: std::sync::OnceLock<Vec<Theme>> = std::sync::OnceLock::new();
    THEMES.get_or_init(|| {
        vec![
            palette! { id: "bold-crimson", name: "Crimson", dark: true, bg: "#A03C3C", fg: "#fff7f0", panel: "#933737", rule: "#b15e5c", link: "#fff7f0", dim: "#f1dbd5", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-tangerine", name: "Tangerine", dark: false, bg: "#C85F3C", fg: "#101416", panel: "#cc6c4c", rule: "#a85337", link: "#101416", dim: "#322726", accent: "#101416", code: "Github" },
            palette! { id: "bold-sunflower", name: "Sunflower", dark: false, bg: "#FFDE30", fg: "#101416", panel: "#ffe141", rule: "#d5bb2d", link: "#101416", dim: "#3a3a24", accent: "#101416", code: "Github" },
            palette! { id: "bold-emerald", name: "Emerald", dark: false, bg: "#388E70", fg: "#101416", panel: "#48977b", rule: "#327a62", link: "#101416", dim: "#1c2e2e", accent: "#101416", code: "Github" },
            palette! { id: "bold-turquoise", name: "Turquoise", dark: false, bg: "#58C9B9", fg: "#101416", panel: "#65cdbf", rule: "#4caa9e", link: "#101416", dim: "#213739", accent: "#101416", code: "Github" },
            palette! { id: "bold-cobalt", name: "Cobalt", dark: true, bg: "#1E3F66", fg: "#fff7f0", panel: "#1c3a5e", rule: "#46607f", link: "#fff7f0", dim: "#dddbdb", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-violet", name: "Violet", dark: true, bg: "#8C5D91", fg: "#fff7f0", panel: "#815685", rule: "#a179a2", link: "#fff7f0", dim: "#eee0e2", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-pink", name: "Pink", dark: false, bg: "#E4707C", fg: "#101416", panel: "#e67b86", rule: "#bf616c", link: "#101416", dim: "#362930", accent: "#101416", code: "Github" },
            palette! { id: "bold-burgundy", name: "Burgundy", dark: true, bg: "#4A1C1C", fg: "#fff7f0", panel: "#441a1a", rule: "#6b4342", link: "#fff7f0", dim: "#e4d6d0", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-mulberry", name: "Mulberry", dark: true, bg: "#8C4356", fg: "#fff7f0", panel: "#813e4f", rule: "#a16372", link: "#fff7f0", dim: "#eedcd9", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-plum", name: "Plum", dark: true, bg: "#5B445F", fg: "#fff7f0", panel: "#543f57", rule: "#796479", link: "#fff7f0", dim: "#e6dcda", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-forest", name: "Forest", dark: true, bg: "#163229", fg: "#fff7f0", panel: "#142e26", rule: "#40554d", link: "#fff7f0", dim: "#dcd9d2", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-deep-teal", name: "Deep Teal", dark: true, bg: "#2A5254", fg: "#fff7f0", panel: "#274b4d", rule: "#507070", link: "#fff7f0", dim: "#dfded9", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-amber", name: "Amber", dark: true, bg: "#A3682C", fg: "#ffffff", panel: "#966028", rule: "#b48352", link: "#ffffff", dim: "#f1e8df", accent: "#ffffff", code: "Base16OceanDark" },
            palette! { id: "bold-chestnut", name: "Chestnut", dark: true, bg: "#5D4037", fg: "#fff7f0", panel: "#563b33", rule: "#7a6158", link: "#fff7f0", dim: "#e7dcd4", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-copper", name: "Copper", dark: true, bg: "#7D4C3D", fg: "#fff7f0", panel: "#734638", rule: "#946b5d", link: "#fff7f0", dim: "#ecddd5", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-midnight-blue", name: "Midnight Blue", dark: true, bg: "#102349", fg: "#fff7f0", panel: "#0f2043", rule: "#3b4967", link: "#fff7f0", dim: "#dbd7d7", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-azure", name: "Azure", dark: false, bg: "#3D7BAD", fg: "#000000", panel: "#4d86b4", rule: "#32658e", link: "#000000", dim: "#09121a", accent: "#000000", code: "Github" },
            palette! { id: "bold-sky-blue", name: "Sky Blue", dark: false, bg: "#689AAF", fg: "#101416", panel: "#74a2b5", rule: "#588293", link: "#101416", dim: "#1d282d", accent: "#101416", code: "Github" },
            palette! { id: "bold-denim", name: "Denim", dark: true, bg: "#4c6486", fg: "#fff7f0", panel: "#465c7b", rule: "#6c7e99", link: "#fff7f0", dim: "#e4e1e0", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-olive", name: "Olive", dark: true, bg: "#615020", fg: "#fff7f0", panel: "#594a1d", rule: "#7d6e45", link: "#fff7f0", dim: "#e7ded1", accent: "#fff7f0", code: "Base16OceanDark" },
            palette! { id: "bold-gold", name: "Gold", dark: false, bg: "#C6A75C", fg: "#101416", panel: "#cbae69", rule: "#a58d4f", link: "#101416", dim: "#2b2a20", accent: "#101416", code: "Github" },
            palette! { id: "bold-black", name: "Black", dark: true, bg: "#000000", fg: "#dcdcdc", panel: "#080808", rule: "#242424", link: "#ededed", dim: "#a0a0a0", accent: "#dcdcdc", code: "Base16OceanDark" },
            palette! { id: "rose-stone", name: "Rose Stone", dark: false, bg: "#FFF7F7", fg: "#4A1C1C", panel: "#FFE4E4", rule: "#FFD4D4", link: "#A03C3C", dim: "#4A1C1C", accent: "#A03C3C", code: "Github" },
            palette! { id: "warm-clay", name: "Warm Clay", dark: false, bg: "#F8E3C4", fg: "#2C1810", panel: "#E6C89B", rule: "#D4B176", link: "#C85F3C", dim: "#2C1810", accent: "#C85F3C", code: "Github" },
            palette! { id: "cocoa", name: "Cocoa", dark: false, bg: "#EFEBE9", fg: "#5D4037", panel: "#D7CCC8", rule: "#BCAAA4", link: "#795548", dim: "#5D4037", accent: "#795548", code: "Github" },
            palette! { id: "parchment", name: "Parchment", dark: false, bg: "#F4E4BE", fg: "#5F4B32", panel: "#E8D5AB", rule: "#E2CCA0", link: "#A3682C", dim: "#5F4B32", accent: "#A3682C", code: "Github" },
            palette! { id: "jade", name: "Jade", dark: false, bg: "#E8F4EE", fg: "#1D4A3C", panel: "#DCEEE5", rule: "#D5EAE0", link: "#388E70", dim: "#1D4A3C", accent: "#388E70", code: "Github" },
            palette! { id: "lagoon", name: "Lagoon", dark: false, bg: "#E6F3F4", fg: "#2A5254", panel: "#D4EBEC", rule: "#C2E3E4", link: "#3C707A", dim: "#2A5254", accent: "#3C707A", code: "Github" },
            palette! { id: "porcelain", name: "Porcelain", dark: false, bg: "#F2F7FF", fg: "#102349", panel: "#D4E5FF", rule: "#B9D7FF", link: "#1E3F66", dim: "#102349", accent: "#1E3F66", code: "Github" },
            palette! { id: "lavender-clay", name: "Lavender Clay", dark: false, bg: "#F4EEF5", fg: "#442A47", panel: "#E8DCEA", rule: "#DBCBDE", link: "#8C5D91", dim: "#442A47", accent: "#8C5D91", code: "Github" },
            palette! { id: "blossom", name: "Blossom", dark: false, bg: "#FFEEF2", fg: "#8C4356", panel: "#FFE6EC", rule: "#FFDEE6", link: "#E4707C", dim: "#8C4356", accent: "#E4707C", code: "Github" },
            palette! { id: "pewter", name: "Pewter", dark: false, bg: "#F3F4F6", fg: "#1F2937", panel: "#E5E7EB", rule: "#D1D5DB", link: "#4B5563", dim: "#1F2937", accent: "#4B5563", code: "Github" },
            palette! { id: "ink", name: "Ink", dark: true, bg: "#1F2937", fg: "#E5E7EB", panel: "#4B5563", rule: "#374151", link: "#4B5563", dim: "#E5E7EB", accent: "#4B5563", code: "Base16OceanDark" },
            palette! { id: "marble", name: "Marble", dark: false, bg: "#F7F6F2", fg: "#2B2926", panel: "#F2F0EB", rule: "#EAE8E3", link: "#9A958E", dim: "#2B2926", accent: "#9A958E", code: "Github" },
            palette! { id: "light", name: "Light", dark: false, bg: "#ffffff", fg: "#24292f", panel: "#f6f8fa", rule: "#d0d7de", link: "#0969da", dim: "#6e7781", accent: "#0969da", code: "Github" },
            palette! { id: "mist", name: "Mist", dark: false, bg: "#c8cfd7", fg: "#283541", panel: "#b9c3ce", rule: "#8c9aaa", link: "#215581", dim: "#3b4b5b", accent: "#276694", code: "Github" },
            palette! { id: "sage", name: "Sage", dark: false, bg: "#becbc3", fg: "#293b33", panel: "#afc0b5", rule: "#869d8f", link: "#285d46", dim: "#354d3e", accent: "#2b6c50", code: "Github" },
            palette! { id: "slate", name: "Slate", dark: true, bg: "#536273", fg: "#f2f5f8", panel: "#475768", rule: "#748699", link: "#c0e5ff", dim: "#d1dce7", accent: "#9dd7ff", code: "Nord" },
            palette! { id: "graphite", name: "Graphite", dark: true, bg: "#57595d", fg: "#f5f5f5", panel: "#4a4c50", rule: "#7a7d82", link: "#c4dff5", dim: "#d5d7db", accent: "#acd2f0", code: "Base16OceanDark" },
            palette! { id: "dark", name: "Dark", dark: true, bg: "#1f1f1f", fg: "#d7d7d7", panel: "#181818", rule: "#343434", link: "#75beff", dim: "#a5a5a5", accent: "#3794ff", code: "Base16OceanDark" },
            palette! { id: "dracula", name: "Dracula", dark: true, bg: "#282a36", fg: "#f8f8f2", panel: "#21222c", rule: "#44475a", link: "#bd93f9", dim: "#6272a4", accent: "#ff79c6", code: "Dracula" },
            palette! { id: "solarized-dark", name: "Solarized Dark", dark: true, bg: "#002b36", fg: "#93a1a1", panel: "#073642", rule: "#094959", link: "#268bd2", dim: "#586e75", accent: "#b58900", code: "SolarizedDark" },
            palette! { id: "solarized-light", name: "Solarized Light", dark: false, bg: "#fdf6e3", fg: "#586e75", panel: "#eee8d5", rule: "#d7cfb8", link: "#268bd2", dim: "#93a1a1", accent: "#b58900", code: "SolarizedLight" },
            palette! { id: "nord", name: "Nord", dark: true, bg: "#2e3440", fg: "#d8dee9", panel: "#3b4252", rule: "#4c566a", link: "#88c0d0", dim: "#7b88a1", accent: "#81a1c1", code: "Nord" },
            palette! { id: "gruvbox", name: "Gruvbox Dark", dark: true, bg: "#282828", fg: "#ebdbb2", panel: "#32302f", rule: "#504945", link: "#83a598", dim: "#928374", accent: "#fabd2f", code: "GruvboxDark" },
            palette! { id: "monokai", name: "Monokai", dark: true, bg: "#272822", fg: "#f8f8f2", panel: "#2f302a", rule: "#49483e", link: "#66d9ef", dim: "#75715e", accent: "#a6e22e", code: "MonokaiExtended" },
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

pub fn blend(foreground: &str, background: &str, amount: f64) -> String {
    let fg = foreground.trim_start_matches('#');
    let bg = background.trim_start_matches('#');
    let c = |index| {
        ((channel(fg, index) * amount + channel(bg, index) * (1.0 - amount)) * 255.0).round() as u8
    };
    format!("#{:02x}{:02x}{:02x}", c(0), c(2), c(4))
}

pub fn guard_surfaces(color: &str, surfaces: &[&str], target: f64) -> String {
    let score = |candidate: &str| {
        surfaces
            .iter()
            .map(|bg| contrast(candidate, bg))
            .fold(f64::INFINITY, f64::min)
    };
    if score(color) >= target {
        return color.into();
    }
    for step in 1..=255 {
        for end in ["#000000", "#ffffff"] {
            let candidate = blend(end, color, f64::from(step) / 255.0);
            if score(&candidate) >= target {
                return candidate;
            }
        }
    }
    if score("#000000") >= score("#ffffff") {
        "#000000".into()
    } else {
        "#ffffff".into()
    }
}

impl Theme {
    pub fn readable(mut self, strength: u32) -> Self {
        self.text_contrast = strength;
        self.accent = guard_surfaces(&self.accent, &[&self.bg, &self.panel], 3.0);
        self.selected = blend(&self.accent, &self.panel, 0.18);
        self.fg = guard_surfaces(
            &strengthen(&self.fg, &self.bg, strength),
            &[&self.bg, &self.panel, &self.selected],
            4.5,
        );
        if self.dim == self.fg {
            self.dim = blend(&self.fg, &self.bg, 0.7);
        }
        self.dim = guard_surfaces(&self.dim, &[&self.bg, &self.panel, &self.selected], 4.5);
        self.link = guard_surfaces(
            &strengthen(&self.link, &self.bg, strength),
            &[&self.bg, &self.panel],
            4.5,
        );
        self.mark_fg = guard(&self.bg, &self.accent, 4.5);
        self.input_rule = guard_surfaces(&self.rule, &[&self.bg, &self.panel], 3.0);
        self
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
