
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
}

fn t(
    id: &str, name: &str, dark: bool, bg: &str, fg: &str, panel: &str, bar: &str,
    rule: &str, link: &str, dim: &str, accent: &str, code: &str,
) -> Theme {
    Theme {
        id: id.into(), name: name.into(), dark,
        bg: bg.into(), fg: fg.into(), panel: panel.into(), bar: bar.into(),
        rule: rule.into(), link: link.into(), dim: dim.into(), accent: accent.into(),
        code: code.into(),
    }
}

pub fn builtin() -> Vec<Theme> {
    vec![
        t("light", "Light", false, "#ffffff", "#24292f", "#f6f8fa", "#f0f3f6",
          "#d0d7de", "#0969da", "#6e7781", "#0969da", "Github"),
        t("dark", "Dark", true, "#2b303b", "#c0c5ce", "#232830", "#1f242c",
          "#4f5b66", "#8fa1b3", "#7a8593", "#8fa1b3", "Base16OceanDark"),
        t("dracula", "Dracula", true, "#282a36", "#f8f8f2", "#21222c", "#191a21",
          "#44475a", "#bd93f9", "#6272a4", "#ff79c6", "Dracula"),
        t("solarized-dark", "Solarized Dark", true, "#002b36", "#93a1a1", "#073642",
          "#00212b", "#094959", "#268bd2", "#586e75", "#b58900", "SolarizedDark"),
        t("solarized-light", "Solarized Light", false, "#fdf6e3", "#586e75", "#eee8d5",
          "#e8e1cc", "#d7cfb8", "#268bd2", "#93a1a1", "#b58900", "SolarizedLight"),
        t("nord", "Nord", true, "#2e3440", "#d8dee9", "#3b4252", "#2b3240",
          "#4c566a", "#88c0d0", "#7b88a1", "#81a1c1", "Nord"),
        t("gruvbox", "Gruvbox Dark", true, "#282828", "#ebdbb2", "#32302f", "#1d2021",
          "#504945", "#83a598", "#928374", "#fabd2f", "GruvboxDark"),
        t("monokai", "Monokai", true, "#272822", "#f8f8f2", "#2f302a", "#1e1f1a",
          "#49483e", "#66d9ef", "#75715e", "#a6e22e", "MonokaiExtended"),
    ]
}

pub fn find(id: &str) -> Theme {
    builtin()
        .into_iter()
        .find(|t| t.id == id)
        .unwrap_or_else(|| builtin().into_iter().find(|t| t.id == "dark").unwrap())
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
    let f = |c: f64| if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
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
    let toward_light = luminance(bg) < 0.5;
    let mut rgb: Vec<f64> = (0..3).map(|i| channel(h, i * 2) * 255.0).collect();
    for _ in 0..40 {
        for c in rgb.iter_mut() {
            *c = if toward_light { (*c + 8.0).min(255.0) } else { (*c - 8.0).max(0.0) };
        }
        let candidate = format!("#{:02x}{:02x}{:02x}", rgb[0] as u8, rgb[1] as u8, rgb[2] as u8);
        if contrast(&candidate, bg) >= target {
            return candidate;
        }
    }
    if toward_light { "#ffffff".into() } else { "#000000".into() }
}
