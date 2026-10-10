use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use slot_cheevos::wifi::{Radio, Watcher};

pub const WIFI_FILE: &str = "Config/wifi.txt";

const POLL: Duration = Duration::from_millis(500);

static WATCHING: AtomicBool = AtomicBool::new(false);
static ONLINE: AtomicBool = AtomicBool::new(false);

pub fn online() -> bool {
    !WATCHING.load(Ordering::Acquire) || ONLINE.load(Ordering::Acquire)
}

pub fn command(root: &Path, verb: &str) -> Command {
    let mut cmd = Command::new("/bin/sh");
    cmd.arg(root.join("System/slotnet.sh"))
        .arg(verb)
        .env("SLOT_ROOT", root);
    cmd
}

pub fn link_marker() -> PathBuf {
    let run = std::env::var_os("AGS_RUN").unwrap_or_else(|| "/run".into());
    Path::new(&run).join("slotlink.session")
}

pub struct Script {
    pub root: PathBuf,
    pub marker: PathBuf,
}

impl Radio for Script {
    fn link_active(&self) -> bool {
        self.marker.exists()
    }

    fn up(&mut self) -> bool {
        match command(&self.root, "up").status() {
            Ok(s) if s.success() => {
                eprintln!("slot: wifi: up");
                true
            }
            Ok(s) => {
                eprintln!("slot: wifi: up failed: {}", why(s.code()));
                false
            }
            Err(e) => {
                eprintln!("slot: wifi: {e}");
                false
            }
        }
    }

    fn down(&mut self) {
        eprintln!("slot: wifi: yielding to link play");
        let _ = command(&self.root, "down").status();
    }
}

pub fn why(code: Option<i32>) -> &'static str {
    match code {
        Some(1) => "could not join the network or get an address",
        Some(3) => "no ssid in Config/wifi.txt",
        Some(4) => "link play has the radio",
        Some(5) => "no DHCP client, set address in Config/wifi.txt",
        _ => "the script failed",
    }
}

pub fn configured(root: &Path) -> bool {
    slot_store::ini::value(root, WIFI_FILE, "ssid").is_some_and(|s| !s.is_empty())
}

pub fn start(root: &Path) {
    if !cfg!(feature = "device") || !configured(root) {
        return;
    }
    if WATCHING.swap(true, Ordering::AcqRel) {
        return;
    }
    let mut radio = Script {
        root: root.to_path_buf(),
        marker: link_marker(),
    };
    let spawned = std::thread::Builder::new()
        .name("slot-net".into())
        .spawn(move || {
            let mut watcher = Watcher::new(Instant::now());
            loop {
                let up = watcher.tick(&mut radio, Instant::now());
                ONLINE.store(up, Ordering::Release);
                std::thread::sleep(POLL);
            }
        });
    if let Err(e) = spawned {
        eprintln!("slot: wifi: {e}");
        WATCHING.store(false, Ordering::Release);
    }
}
