// What changes from one edition (or one distribution) to another lives in a
// data file, not in the code: /usr/share/soplos-linux-installer/editions/
// <codename>.json (DESIGN.md, section 10). Another distribution that uses the
// installer only has to ship its own file. The files of the Soplos editions
// are also built into the program, so it works from the source tree too.

use serde::Deserialize;
use std::fs;

const EDITIONS_DIR: &str = "/usr/share/soplos-linux-installer/editions";

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Requirements {
    /// Size of the disk, in GB (10^9 bytes, as disks are sold).
    pub disk_gb: Option<f64>,
    /// Memory, in GiB as the kernel counts it: a 2 GiB machine reports a bit
    /// less than 2 in /proc/meminfo, so the figures leave room for that.
    pub memory_gb: Option<f64>,
}

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditionData {
    #[allow(dead_code)]
    pub codename: String,
    pub name: String,
    #[serde(default)]
    pub requirements: Requirements,
    /// An address of the servers of the distribution: any answer proves that
    /// the packages can be downloaded.
    pub internet_probe: Option<String>,
    /// A geolocation service that answers {"time_zone": "Europe/Madrid"} (the
    /// one Calamares uses). It picks the time zone the map starts on.
    pub geoip_url: Option<String>,
    /// "lightdm", "gdm3" or "plasmalogin": whose live autologin is removed
    /// and what is set up for it (DESIGN.md, section 10).
    #[serde(default)]
    pub display_manager: String,
    /// Packages of the live session, purged from the installed system when
    /// they are there.
    #[serde(default)]
    pub remove_packages: Vec<String>,
    /// Packages installed from the internet (soplos-welcome).
    #[serde(default)]
    pub install_packages: Vec<String>,
    /// LightDM: the picture copied as the greeter background.
    #[serde(default)]
    pub lightdm_background: Option<String>,
    /// Live leftovers deleted at the end: the Calamares files and scripts,
    /// sudo rules of the live user, /etc/apt/sources.list.
    #[serde(default)]
    pub remove_files: Vec<String>,
}

fn built_in(codename: &str) -> Option<&'static str> {
    match codename {
        "boro" => Some(include_str!("../resources/editions/boro.json")),
        "tyron" => Some(include_str!("../resources/editions/tyron.json")),
        "tyson" => Some(include_str!("../resources/editions/tyson.json")),
        _ => None,
    }
}

/// The data of an edition: the installed file wins over the built-in one.
/// An unknown edition has no requirements and no probe address.
pub fn load(codename: &str) -> EditionData {
    let path = format!("{EDITIONS_DIR}/{codename}.json");
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(data) = serde_json::from_str(&text) {
            return data;
        }
    }
    built_in(codename)
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_editions_parse() {
        for codename in ["boro", "tyron", "tyson"] {
            let data: EditionData = serde_json::from_str(built_in(codename).unwrap()).unwrap();
            assert_eq!(data.codename, codename);
            assert!(data.requirements.disk_gb.is_some() && data.requirements.memory_gb.is_some());
        }
    }
}
