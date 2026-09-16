use egui::FontFamily;
use std::path::{Path, PathBuf};

/// Prefer Tahoma for UI text; fall back to Verdana / Liberation / Segoe.
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let sans_candidates = [
        "C:\\Windows\\Fonts\\verdana.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/TTF/LiberationSans-Regular.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];
    let mono_candidates = [
        "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/TTF/LiberationMono-Regular.ttf",
        "C:\\Windows\\Fonts\\lucon.ttf",
        "C:\\Windows\\Fonts\\consola.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    ];

    let sans_data = load_tahoma().or_else(|| load_first_font(&sans_candidates));
    if let Some(data) = sans_data {
        fonts
            .font_data
            .insert("slsb_sans".into(), egui::FontData::from_owned(data).into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "slsb_sans".into());
    }
    if let Some(data) = load_first_font(&mono_candidates) {
        fonts
            .font_data
            .insert("slsb_mono".into(), egui::FontData::from_owned(data).into());
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "slsb_mono".into());
    }

    // Tahoma / Liberation omit dingbats (✎ ✕ ⚠ ◆). egui's last fallback
    // (emoji-icon-font) maps those codepoints to empty boxes, so icons look
    // like tofu unless a real symbol font sits ahead of it.
    let symbol_candidates = [
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/noto/NotoSansSymbols2-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
        "/usr/share/fonts/noto/NotoSansSymbols-Regular.ttf",
        "C:\\Windows\\Fonts\\seguisym.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
    ];
    if let Some(data) = load_first_font(&symbol_candidates) {
        fonts.font_data.insert(
            "slsb_symbols".into(),
            egui::FontData::from_owned(data).into(),
        );
        prepend_after_primary(
            fonts.families.entry(FontFamily::Proportional).or_default(),
            "slsb_sans",
            "slsb_symbols",
        );
        prepend_after_primary(
            fonts.families.entry(FontFamily::Monospace).or_default(),
            "slsb_mono",
            "slsb_symbols",
        );
    }

    ctx.set_fonts(fonts);
}

/// Put `fallback` just after the custom primary face so Latin stays Tahoma
/// (or similar) while missing glyphs resolve before egui's box font.
fn prepend_after_primary(family: &mut Vec<String>, primary: &str, fallback: &str) {
    let idx = usize::from(family.first().map(String::as_str) == Some(primary));
    family.insert(idx, fallback.into());
}

fn load_tahoma() -> Option<Vec<u8>> {
    let mut paths: Vec<PathBuf> = vec![
        PathBuf::from(r"C:\Windows\Fonts\tahoma.ttf"),
        PathBuf::from("/usr/share/fonts/TTF/tahoma.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/msttcorefonts/Tahoma.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/msttcorefonts/tahoma.ttf"),
        PathBuf::from("/usr/share/wine/fonts/tahoma.ttf"),
        PathBuf::from("/usr/share/fonts/wine/tahoma.ttf"),
    ];
    if let Ok(windir) = std::env::var("WINDIR") {
        paths.push(PathBuf::from(windir).join("Fonts").join("tahoma.ttf"));
    }
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".fonts/tahoma.ttf"));
        paths.push(home.join(".local/share/fonts/tahoma.ttf"));
        paths.push(home.join(".wine/drive_c/windows/fonts/tahoma.ttf"));
        for compat in [
            home.join(".local/share/Steam/steamapps/compatdata"),
            home.join(".steam/steam/steamapps/compatdata"),
            home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata"),
        ] {
            if let Some(found) = tahoma_in_compatdata(&compat) {
                paths.push(found);
            }
        }
    }
    for path in &paths {
        if let Ok(bytes) = std::fs::read(path) {
            if !bytes.is_empty() {
                return Some(bytes);
            }
        }
    }
    fontconfig_family("Tahoma")
}

fn tahoma_in_compatdata(root: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten().take(80) {
        let font = entry.path().join("pfx/drive_c/windows/fonts/tahoma.ttf");
        if font.is_file() {
            return Some(font);
        }
    }
    None
}

/// `fc-match` only if it actually resolved to `family` (not a substitute).
fn fontconfig_family(family: &str) -> Option<Vec<u8>> {
    let out = std::process::Command::new("fc-match")
        .args(["-f", "%{family}\n%{file}", family])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let resolved = lines.next()?.trim();
    if !resolved.eq_ignore_ascii_case(family) {
        return None;
    }
    let file = lines.next()?.trim();
    let bytes = std::fs::read(file).ok()?;
    (!bytes.is_empty()).then_some(bytes)
}

fn load_first_font(paths: &[&str]) -> Option<Vec<u8>> {
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            if !bytes.is_empty() {
                return Some(bytes);
            }
        }
    }
    None
}
