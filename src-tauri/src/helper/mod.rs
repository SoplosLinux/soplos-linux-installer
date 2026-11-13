// The helper: the part of the installer that runs as root and does the disk
// work. The window starts it with `sudo -n soplos-linux-installer --helper`,
// writes the installation plan as one JSON document on its stdin, and reads
// its progress as JSON lines on its stdout (DESIGN.md, sections 2 and 6).
//
// With --dry-run it changes nothing: it reads the disk and lists the commands
// it would run, so a plan can be checked before it touches a disk.
//
// Every step of section 6, the drivers of section 8 included.

pub mod disk;
pub mod drivers;
pub mod exec;
pub mod kernel;
pub mod nvidia_patches;
pub mod plan;
pub mod system;

use serde::Serialize;
use std::io::{self, Read, Write};

/// One line of progress, as the window reads it.
#[derive(Serialize, Debug)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    /// A step starts: its id and its position among all the steps.
    Step { id: String, index: usize, total: usize },
    /// Progress of the whole installation, from 0 to 100.
    Progress { percent: u8 },
    /// A line for the log under "Show details".
    Log { line: String },
    /// The end: success, or the reason it stopped.
    Done {
        success: bool,
        message: String,
        #[serde(rename = "dryRun")]
        dry_run: bool,
    },
}

fn emit(event: &Event) {
    let mut out = io::stdout().lock();
    if let Ok(text) = serde_json::to_string(event) {
        let _ = writeln!(out, "{text}");
        let _ = out.flush();
    }
}

/// The steps in order, each with the percentage the bar is at when it
/// starts. The copy takes most of the time and moves the bar by itself.
const STEPS: [(&str, u8); 11] = [
    ("partition", 0),
    ("format", 3),
    ("copy", 6),
    ("configure", 56),
    ("users", 60),
    ("packages", 63),
    ("kernel", 68),
    ("drivers", 74),
    ("bootloader", 84),
    ("finish", 95),
    ("unmount", 98),
];

/// Runs the helper and returns the exit code of the process.
pub fn run() -> i32 {
    let dry_run = std::env::args().any(|a| a == "--dry-run");
    let done = |success: bool, message: String| emit(&Event::Done { success, message, dry_run });

    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        done(false, format!("could not read the plan: {e}"));
        return 1;
    }
    let plan: plan::Plan = match serde_json::from_str(&input) {
        Ok(plan) => plan,
        Err(e) => {
            done(false, format!("the plan is not valid: {e}"));
            return 1;
        }
    };

    let mut ctx = exec::Ctx::new(dry_run, Box::new(emit));
    if !dry_run {
        let uid = ctx.read("id", &["-u"]).unwrap_or_default();
        if uid.trim() != "0" {
            done(false, "the helper must run as root".into());
            return 1;
        }
    }
    ctx.log(&format!(
        "Soplos Linux Installer {} — {}",
        env!("CARGO_PKG_VERSION"),
        if dry_run { "dry run: nothing is changed" } else { "installation" }
    ));

    let step = |ctx: &exec::Ctx, id: &str| {
        let index = STEPS.iter().position(|(s, _)| *s == id).unwrap_or(0);
        ctx.emit(&Event::Step { id: id.into(), index, total: STEPS.len() });
        ctx.emit(&Event::Progress { percent: STEPS[index].1 });
    };
    let start_of = |id: &str| STEPS.iter().find(|(s, _)| *s == id).map(|(_, p)| *p).unwrap_or(0);

    let edition = crate::edition_data::load(&crate::commands::environment::edition());
    let mut volumes = disk::Volumes::default();
    let result = (|| -> Result<(), String> {
        let state = disk::read_state(&mut ctx, &plan.disk.device)?;
        step(&ctx, "partition");
        let placed = disk::partition(&mut ctx, &plan, &state)?;
        step(&ctx, "format");
        volumes = disk::format_and_mount(&mut ctx, &plan, &placed, &state)?;
        step(&ctx, "copy");
        system::copy_system(&mut ctx, start_of("copy"), start_of("configure") - 1)?;
        system::enter(&mut ctx, plan.firmware != "bios")?;
        step(&ctx, "configure");
        let mut info = system::write_tables(&mut ctx, &volumes, plan.disk.swap == "hibernate")?;
        system::configure(&mut ctx, &plan)?;
        step(&ctx, "users");
        system::users(&mut ctx, &plan)?;
        system::display_manager(&mut ctx, &edition, &plan)?;
        step(&ctx, "packages");
        system::packages(&mut ctx, &edition, start_of("packages"), start_of("kernel") - 1)?;
        step(&ctx, "kernel");
        kernel::install(&mut ctx);
        step(&ctx, "drivers");
        if drivers::install(&mut ctx) {
            info.extra.push("nvidia-drm.modeset=1".into());
        }
        step(&ctx, "bootloader");
        system::bootloader(&mut ctx, &plan, &edition, &info, plan.disk.mode == "alongside")?;
        step(&ctx, "finish");
        system::finish(&mut ctx, &edition)?;
        Ok(())
    })();

    step(&ctx, "unmount");
    disk::release(&mut ctx, &volumes);
    ctx.emit(&Event::Progress { percent: 100 });

    match result {
        Ok(()) => {
            done(true, String::new());
            0
        }
        Err(message) => {
            ctx.log(&format!("ERROR: {message}"));
            done(false, message);
            1
        }
    }
}
