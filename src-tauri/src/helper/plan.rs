// The installation plan the window sends on stdin: the same shape as
// InstallPlan in src/lib/wizard.ts. Fields the helper does not use yet are
// ignored; missing ones take their defaults.

use serde::Deserialize;

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Plan {
    pub locale: String,
    pub formats: String,
    pub timezone: String,
    pub keyboard: Keyboard,
    /// "uefi" or "bios".
    pub firmware: String,
    /// "grub", "systemd-boot" or "limine".
    pub bootloader: String,
    pub disk: DiskPlan,
    pub user: UserPlan,
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
pub struct Keyboard {
    pub layout: String,
    pub variant: String,
}

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct DiskPlan {
    pub device: String,
    /// "erase", "alongside" or "manual".
    pub mode: String,
    pub filesystem: String,
    pub encrypt: bool,
    pub passphrase: String,
    /// "none", "swap" or "hibernate".
    pub swap: String,
    pub layout: Option<Layout>,
}

#[derive(Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Layout {
    pub new_table: bool,
    /// "gpt" or "msdos", with new_table.
    pub table: Option<String>,
    pub parts: Vec<Part>,
}

#[derive(Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Part {
    /// The existing partition, "" for a new one.
    pub path: String,
    /// Bytes from the start of the disk.
    pub start: u64,
    pub size: u64,
    /// "keep", "shrink", "use", "format" or "create".
    pub action: String,
    pub fs: String,
    pub mount: String,
    pub parttype: String,
    pub encrypt: bool,
    pub label: String,
}

impl Part {
    /// Whether a new file system is written on it.
    pub fn writes(&self) -> bool {
        self.action == "create" || self.action == "format"
    }
}

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct UserPlan {
    pub full_name: String,
    pub login: String,
    pub hostname: String,
    pub password: String,
    pub same_root_password: bool,
    pub root_password: String,
}

pub const GUID_ESP: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";
pub const GUID_BIOS_GRUB: &str = "21686148-6449-6e6f-744e-656564454649";
pub const GUID_XBOOTLDR: &str = "bc13c2ff-59e6-4262-a352-b275fd6f7172";
pub const GUID_SWAP: &str = "0657fd6d-a4ab-43c4-84e5-0933c84b4f4f";
pub const GUID_BASIC_DATA: &str = "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7";
