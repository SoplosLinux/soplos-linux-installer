// The data of the location step: the time zones with their place on the map,
// the languages the system can be installed in and the keyboard layouts. All
// of it comes from the files of the live system, which are the files of the
// system being installed (tzdata, locales, xkeyboard-config).

use crate::commands::environment;
use crate::edition_data;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::process::{Command, Stdio};

const ZONE_TAB: &str = "/usr/share/zoneinfo/zone.tab";
const SUPPORTED_LOCALES: &str = "/usr/share/i18n/SUPPORTED";
const XKB_LIST: &str = "/usr/share/X11/xkb/rules/evdev.lst";

#[derive(Serialize)]
pub struct Zone {
    /// "Europe/Madrid".
    pub id: String,
    /// ISO 3166-1 alpha-2 code of the country, "ES".
    pub country: String,
    pub lat: f64,
    pub lon: f64,
}

/// "+4024" or "+402415" (degrees, minutes, seconds) as decimal degrees.
fn dms(text: &str, degree_digits: usize) -> Option<f64> {
    let sign = if text.starts_with('-') { -1.0 } else { 1.0 };
    let digits = &text[1..];
    let degrees: f64 = digits.get(..degree_digits)?.parse().ok()?;
    let minutes: f64 = digits.get(degree_digits..degree_digits + 2)?.parse().ok()?;
    let seconds: f64 = digits.get(degree_digits + 2..).filter(|s| !s.is_empty()).map_or(Some(0.0), |s| s.parse().ok())?;
    Some(sign * (degrees + minutes / 60.0 + seconds / 3600.0))
}

/// "+4024-00341": the latitude, then the longitude from the second sign on.
fn coordinates(text: &str) -> Option<(f64, f64)> {
    let split = text[1..].find(['+', '-'])? + 1;
    Some((dms(&text[..split], 2)?, dms(&text[split..], 3)?))
}

fn parse_zones(text: &str) -> Vec<Zone> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let (lat, lon) = coordinates(fields.get(1)?)?;
            Some(Zone { id: fields.get(2)?.to_string(), country: fields.first()?.to_string(), lat, lon })
        })
        .collect()
}

/// The UTF-8 locales of glibc: "es_ES.UTF-8", "ca_ES.UTF-8@valencia".
fn parse_locales(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            (parts.next()? == "UTF-8").then(|| name.to_string())
        })
        .collect()
}

#[derive(Serialize)]
pub struct Variant {
    pub name: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct Layout {
    pub name: String,
    pub description: String,
    pub variants: Vec<Variant>,
}

/// The "! layout" and "! variant" sections of evdev.lst. A variant line is
/// "  cat             es: Catalan (Spain, with middle-dot L)".
fn parse_layouts(text: &str, translate: &dyn Fn(&str) -> String) -> Vec<Layout> {
    let mut layouts: BTreeMap<String, Layout> = BTreeMap::new();
    let mut section = "";
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("! ") {
            section = if name.trim() == "layout" { "layout" } else if name.trim() == "variant" { "variant" } else { "" };
            continue;
        }
        let line = line.trim();
        let Some((name, rest)) = line.split_once(char::is_whitespace) else { continue };
        let rest = rest.trim();
        match section {
            "layout" => {
                layouts.insert(
                    name.to_string(),
                    Layout { name: name.to_string(), description: translate(rest), variants: Vec::new() },
                );
            }
            "variant" => {
                if let Some((layout, description)) = rest.split_once(": ") {
                    if let Some(entry) = layouts.get_mut(layout) {
                        entry.variants.push(Variant { name: name.to_string(), description: translate(description) });
                    }
                }
            }
            _ => {}
        }
    }
    let mut list: Vec<Layout> = layouts.into_values().collect();
    for layout in &mut list {
        layout.variants.sort_by_key(|v| v.description.to_lowercase());
    }
    list
}

/// The translations of xkeyboard-config for a language ("es"). Newer
/// releases use the domain "xkeyboard-config-2".
fn xkb_catalog(language: &str) -> Option<gettext::Catalog> {
    ["xkeyboard-config-2", "xkeyboard-config"].iter().find_map(|domain| {
        let file = fs::File::open(format!("/usr/share/locale/{language}/LC_MESSAGES/{domain}.mo")).ok()?;
        gettext::Catalog::parse(file).ok()
    })
}

#[derive(Serialize)]
pub struct LocationData {
    pub zones: Vec<Zone>,
    pub locales: Vec<String>,
}

#[tauri::command(async)]
pub fn get_location_data() -> LocationData {
    LocationData {
        zones: parse_zones(&fs::read_to_string(ZONE_TAB).unwrap_or_default()),
        locales: parse_locales(&fs::read_to_string(SUPPORTED_LOCALES).unwrap_or_default()),
    }
}

/// The keyboard layouts, described in the language of the installer.
#[tauri::command(async)]
pub fn get_keyboard_layouts(language: String) -> Vec<Layout> {
    let catalog = if language == "en" { None } else { xkb_catalog(&language) };
    let translate = |text: &str| catalog.as_ref().map_or_else(|| text.to_string(), |c| c.gettext(text).to_string());
    let mut list = parse_layouts(&fs::read_to_string(XKB_LIST).unwrap_or_default(), &translate);
    list.sort_by_key(|l| l.description.to_lowercase());
    list
}

/// Where the computer is: the geolocation service of the edition file, else
/// the zone of the live system when it is not plain UTC. None: unknown.
#[tauri::command(async)]
pub fn detect_timezone() -> Option<String> {
    let data = edition_data::load(&environment::edition());
    if let Some(url) = data.geoip_url.as_deref() {
        let answer = Command::new("curl")
            .args(["-s", "-m", "6", "--connect-timeout", "5", url])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();
        if let Ok(output) = answer {
            let zone = serde_json::from_slice::<serde_json::Value>(&output.stdout)
                .ok()
                .and_then(|v| v.get("time_zone").and_then(|z| z.as_str()).map(str::to_string));
            if let Some(zone) = zone.filter(|z| z.contains('/')) {
                return Some(zone);
            }
        }
    }
    let link = fs::read_link("/etc/localtime").ok()?;
    let zone = link.to_string_lossy().split("zoneinfo/").nth(1)?.to_string();
    (zone.contains('/') && !zone.starts_with("Etc/")).then_some(zone)
}

/// Puts the chosen layout on the keyboard of the live session, so it can be
/// tried in the test field. X11 takes setxkbmap; on Wayland each desktop
/// keeps its own setting.
#[tauri::command(async)]
pub fn set_live_keyboard(layout: String, variant: String) -> Result<(), String> {
    let valid = |s: &str| s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if layout.is_empty() || !valid(&layout) || !valid(&variant) {
        return Err("invalid layout".into());
    }
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    let run = |program: &str, args: &[&str]| -> Result<(), String> {
        let status = Command::new(program).args(args).stdin(Stdio::null()).status().map_err(|e| e.to_string())?;
        if status.success() { Ok(()) } else { Err(format!("{program} failed")) }
    };
    if session != "wayland" {
        let mut args = vec!["-layout", layout.as_str()];
        if !variant.is_empty() {
            args.extend(["-variant", variant.as_str()]);
        }
        return run("setxkbmap", &args);
    }
    if desktop.contains("gnome") {
        let source = if variant.is_empty() { layout.clone() } else { format!("{layout}+{variant}") };
        return run("gsettings", &["set", "org.gnome.desktop.input-sources", "sources", &format!("[('xkb', '{source}')]")]);
    }
    if desktop.contains("kde") || desktop.contains("plasma") {
        for (key, value) in [("Use", "true"), ("LayoutList", layout.as_str()), ("VariantList", variant.as_str())] {
            run("kwriteconfig6", &["--file", "kxkbrc", "--group", "Layout", "--key", key, value])?;
        }
        return run("dbus-send", &["--session", "--type=signal", "/Layouts", "org.kde.keyboard.reloadConfig"]);
    }
    Err("unsupported session".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_of_zone_tab() {
        let (lat, lon) = coordinates("+4024-00341").unwrap();
        assert!((lat - 40.4).abs() < 1e-9 && (lon + (3.0 + 41.0 / 60.0)).abs() < 1e-9);
        let (lat, lon) = coordinates("-332215-0703910").unwrap();
        assert!(lat < -33.37 && lat > -33.38 && lon < -70.65 && lon > -70.66);
    }

    #[test]
    fn layouts_and_variants() {
        let text = "! model\n  pc105 Generic\n\n! layout\n  es              Spanish\n  us              English (US)\n\n! variant\n  cat             es: Catalan (Spain, with middle-dot L)\n  intl            us: English (US, intl., with dead keys)\n\n! option\n  grp             Switching\n";
        let layouts = parse_layouts(text, &|s| s.to_string());
        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0].name, "es");
        assert_eq!(layouts[0].variants[0].name, "cat");
        assert_eq!(layouts[1].variants[0].description, "English (US, intl., with dead keys)");
    }

    #[test]
    fn the_real_files() {
        if let Ok(text) = fs::read_to_string(ZONE_TAB) {
            let zones = parse_zones(&text);
            assert!(zones.iter().any(|z| z.id == "Europe/Madrid" && z.country == "ES"));
        }
        if let Ok(text) = fs::read_to_string(SUPPORTED_LOCALES) {
            assert!(parse_locales(&text).iter().any(|l| l == "es_ES.UTF-8"));
        }
        let es = get_keyboard_layouts("es".into());
        if !es.is_empty() {
            assert!(es.iter().any(|l| l.name == "es"));
        }
    }
}
