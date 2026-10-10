use slot_retro::{ButtonMask, LibretroCore, MockCore, RetroCore};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static CORE_LOCK: Mutex<()> = Mutex::new(());

const IWRAM: usize = 0x3000000;

fn lock() -> std::sync::MutexGuard<'static, ()> {
    CORE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn vendored(name: &str) -> Option<LibretroCore> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor")
        .join(format!(
            "{name}_libretro.{}",
            std::env::consts::DLL_EXTENSION
        ));
    if !p.exists() {
        eprintln!("skipping: no {}", p.display());
        return None;
    }
    Some(LibretroCore::open(&p).expect("vendored core is present but would not open"))
}

fn test_rom() -> PathBuf {
    let mut rom = vec![0u8; 0x8000];
    rom[0..4].copy_from_slice(&0xea00002eu32.to_le_bytes());
    rom[0xa0..0xac].copy_from_slice(b"SLOT MEMMAP\0");
    rom[0xac..0xb0].copy_from_slice(b"SLMM");
    rom[0xb0..0xb2].copy_from_slice(b"00");
    rom[0xb2] = 0x96;
    let sum = rom[0xa0..0xbd].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    rom[0xbd] = 0u8.wrapping_sub(sum).wrapping_sub(0x19);
    rom[0xc0..0xc4].copy_from_slice(&0xeafffffeu32.to_le_bytes());
    let p = Path::new(env!("CARGO_TARGET_TMPDIR")).join("slot-memmap.gba");
    std::fs::write(&p, rom).expect("write test rom");
    p
}

fn maps_iwram(core: &dyn RetroCore) -> bool {
    core.memory_map()
        .iter()
        .any(|d| d.start == IWRAM && d.ptr != 0 && d.len >= 0x8000)
}

#[test]
fn mgba_declares_its_memory_map_once_it_has_run_a_frame() {
    let _l = lock();
    let Some(mut core) = vendored("mgba") else {
        return;
    };
    core.load(&test_rom()).unwrap();
    core.run_frame(ButtonMask(0));
    assert!(maps_iwram(&core), "{:x?}", core.memory_map());
}

#[test]
fn gpsp_declares_its_memory_map() {
    let _l = lock();
    let Some(mut core) = vendored("gpsp") else {
        return;
    };
    core.load(&test_rom()).unwrap();
    core.run_frame(ButtonMask(0));
    assert!(maps_iwram(&core), "{:x?}", core.memory_map());
}

#[test]
fn the_mock_core_has_no_memory_map() {
    assert!(MockCore::new().memory_map().is_empty());
}
