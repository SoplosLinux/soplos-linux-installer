// How the helper runs commands. Every command goes to the log, and to the
// window as a log line; what goes on stdin (passphrases) never does. In a dry
// run nothing that changes the system is run: the commands are only listed.

use super::Event;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

pub const LOG_FILE: &str = "/var/log/soplos-linux-installer.log";

pub struct Ctx {
    pub dry_run: bool,
    log: Option<File>,
    emit: Box<dyn Fn(&Event)>,
}

impl Ctx {
    pub fn new(dry_run: bool, emit: Box<dyn Fn(&Event)>) -> Ctx {
        let log = if dry_run {
            None
        } else {
            OpenOptions::new().create(true).append(true).open(LOG_FILE).ok()
        };
        Ctx { dry_run, log, emit }
    }

    pub fn emit(&self, event: &Event) {
        (self.emit)(event);
    }

    pub fn log(&mut self, line: &str) {
        if let Some(file) = self.log.as_mut() {
            let _ = writeln!(file, "{line}");
        }
        self.emit(&Event::Log { line: line.to_string() });
    }

    fn describe(program: &str, args: &[&str], stdin: Option<&Stdin>) -> String {
        let mut text = format!("$ {program} {}", args.join(" "));
        match stdin {
            Some(Stdin::Secret(_)) => text.push_str("  <<< (passphrase)"),
            Some(Stdin::Text(t)) => {
                for line in t.lines() {
                    text.push_str(&format!("\n    {line}"));
                }
            }
            None => {}
        }
        text
    }

    /// Runs a command that changes the system. In a dry run it is only listed.
    pub fn run(&mut self, program: &str, args: &[&str], stdin: Option<Stdin>) -> Result<String, String> {
        let text = Ctx::describe(program, args, stdin.as_ref());
        self.log(&text);
        if self.dry_run {
            return Ok(String::new());
        }
        self.execute(program, args, stdin, 0)
    }

    /// Like run, for commands whose exit codes up to `max_ok` mean success
    /// (e2fsck answers 1 when it fixed something).
    pub fn run_allowing(&mut self, program: &str, args: &[&str], max_ok: i32) -> Result<String, String> {
        let text = Ctx::describe(program, args, None);
        self.log(&text);
        if self.dry_run {
            return Ok(String::new());
        }
        self.execute(program, args, None, max_ok)
    }

    /// Runs a command that only reads, also in a dry run.
    pub fn read(&mut self, program: &str, args: &[&str]) -> Result<String, String> {
        self.execute(program, args, None, 0)
    }

    fn execute(&mut self, program: &str, args: &[&str], stdin: Option<Stdin>, max_ok: i32) -> Result<String, String> {
        let mut child = Command::new(program)
            .args(args)
            .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
            .env("LC_ALL", "C")
            .env("DEBIAN_FRONTEND", "noninteractive")
            .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("{program}: {e}"))?;
        if let (Some(input), Some(mut pipe)) = (stdin, child.stdin.take()) {
            let bytes = match input {
                Stdin::Secret(s) | Stdin::Text(s) => s,
            };
            let _ = pipe.write_all(bytes.as_bytes());
        }
        let output = child.wait_with_output().map_err(|e| format!("{program}: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if output.status.code().is_some_and(|code| code >= 0 && code <= max_ok) {
            Ok(stdout)
        } else {
            let detail = stderr.trim();
            let message = format!("{program} failed ({}): {}", output.status, if detail.is_empty() { stdout.trim() } else { detail });
            if let Some(file) = self.log.as_mut() {
                let _ = writeln!(file, "{message}");
            }
            Err(message)
        }
    }
}

impl Ctx {
    /// Runs a long command and passes its output on as it comes: lines that
    /// are a percentage (unsquashfs -percentage) move the bar between `from`
    /// and `to`; the rest go to the log, apt's included.
    pub fn run_progress(&mut self, program: &str, args: &[&str], from: u8, to: u8) -> Result<(), String> {
        let text = Ctx::describe(program, args, None);
        self.log(&text);
        if self.dry_run {
            return Ok(());
        }
        let mut child = Command::new(program)
            .args(args)
            .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
            .env("LC_ALL", "C")
            .env("DEBIAN_FRONTEND", "noninteractive")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("{program}: {e}"))?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        // stderr is read on its own thread, so a chatty command can never
        // block on a full pipe while stdout is being read.
        let stderr = child.stderr.take().ok_or("no stderr")?;
        let errors = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = std::io::Read::read_to_string(&mut BufReader::new(stderr), &mut text);
            text
        });
        let mut last = from;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let trimmed = line.trim();
            if let Ok(percent) = trimmed.parse::<u32>() {
                let value = from as u32 + (to - from) as u32 * percent.min(100) / 100;
                if value as u8 != last {
                    last = value as u8;
                    self.emit(&Event::Progress { percent: last });
                }
            } else if !trimmed.is_empty() {
                self.log(&format!("    {trimmed}"));
            }
        }
        let status = child.wait().map_err(|e| format!("{program}: {e}"))?;
        let detail = errors.join().unwrap_or_default();
        if status.success() {
            Ok(())
        } else {
            let detail = detail.trim().lines().last().unwrap_or("").to_string();
            let message = format!("{program} failed ({status}): {detail}");
            if let Some(file) = self.log.as_mut() {
                let _ = writeln!(file, "{message}");
            }
            Err(message)
        }
    }

    /// Writes a file of the system being installed. In a dry run its
    /// content is only listed.
    pub fn write(&mut self, path: &str, content: &str) -> Result<(), String> {
        let mut text = format!("> {path}");
        for line in content.lines() {
            text.push_str(&format!("\n    {line}"));
        }
        self.log(&text);
        if self.dry_run {
            return Ok(());
        }
        if let Some(dir) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(path, content).map_err(|e| format!("{path}: {e}"))
    }

    /// Changes a file in place with `change`. A missing file is left alone.
    pub fn edit(&mut self, path: &str, what: &str, change: impl Fn(&str) -> String) -> Result<(), String> {
        self.log(&format!("~ {path}: {what}"));
        if self.dry_run {
            return Ok(());
        }
        let Ok(current) = std::fs::read_to_string(path) else { return Ok(()) };
        std::fs::write(path, change(&current)).map_err(|e| format!("{path}: {e}"))
    }

    /// Runs a command inside the system being installed.
    pub fn chroot(&mut self, args: &[&str], stdin: Option<Stdin>) -> Result<String, String> {
        let mut all = vec![super::disk::TARGET];
        all.extend_from_slice(args);
        self.run("chroot", &all, stdin)
    }
}

/// What a command gets on stdin.
pub enum Stdin {
    /// Shown in the log (an sfdisk script).
    Text(String),
    /// Never shown (a passphrase).
    Secret(String),
}
