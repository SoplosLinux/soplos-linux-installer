// The rules of the user step: the login and computer names the system
// accepts, and what the installer proposes from the full name.

/** Names that already exist in the system or belong to the live session. */
const RESERVED = new Set([
  "root", "daemon", "bin", "sys", "sync", "games", "man", "lp", "mail", "news", "uucp", "proxy",
  "www-data", "backup", "list", "irc", "nobody", "systemd-network", "systemd-resolve",
  "systemd-timesync", "messagebus", "avahi", "polkitd", "sddm", "lightdm", "gdm", "liveuser",
  "admin", "sudo", "adm", "users", "video", "audio", "plugdev", "netdev", "cdrom",
]);

/** "José Pérez" -> "jose": the first name, without accents or symbols. */
export function loginFrom(fullName: string): string {
  const first = fullName.trim().split(/\s+/)[0] ?? "";
  return first
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9_-]/g, "")
    .replace(/^[^a-z_]+/, "")
    .slice(0, 32);
}

/** "jose" + "tyron" -> "jose-tyron". */
export function hostnameFrom(login: string, edition: string): string {
  const base = [login, edition || "soplos"].filter(Boolean).join("-");
  return base.replace(/[^a-z0-9-]/g, "").slice(0, 63).replace(/-+$/, "");
}

/** null when valid, else the i18n key of the problem. */
export function loginProblem(login: string): string | null {
  if (!login) return "loginEmpty";
  if (!/^[a-z_][a-z0-9_-]*$/.test(login)) return "loginChars";
  if (login.length > 32) return "loginLong";
  if (RESERVED.has(login)) return "loginReserved";
  return null;
}

export function hostnameProblem(hostname: string): string | null {
  if (!hostname) return "hostnameEmpty";
  if (!/^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/i.test(hostname)) return "hostnameChars";
  if (hostname.length > 63) return "hostnameLong";
  return null;
}

/** 0 to 3: a rough guide, it does not stop anything. */
export function strength(password: string): number {
  if (!password) return 0;
  let classes = 0;
  if (/[a-z]/.test(password)) classes++;
  if (/[A-Z]/.test(password)) classes++;
  if (/[0-9]/.test(password)) classes++;
  if (/[^a-zA-Z0-9]/.test(password)) classes++;
  if (password.length >= 12 && classes >= 3) return 3;
  if (password.length >= 8 && classes >= 2) return 2;
  return 1;
}
