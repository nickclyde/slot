# Fork notes

This fork adds RetroAchievements (softcore) and a home Wi-Fi client to slot. It is kept as a small set of
new files plus one-line hooks in upstream files, so syncing with `origin` stays easy.

## Changes in this fork

- Earn RetroAchievements while playing: sign in from `Config/retroachievements.txt`, identify the inserted
  cart, and show a banner with the badge for every unlock. Softcore only. Banners sit in the top left and
  wait out the BIOS intro on a game's first boot.
- Join a home Wi-Fi network from `Config/wifi.txt`, yielding the radio to link play while it runs.

Setup for players is in `site/retroachievements.html`. Status, decisions and open work for whoever
picks this up next are in `HANDOFF.md`.

## Where the code lives

| Path | What |
|---|---|
| `crates/slot-cheevos/` | rcheevos (vendored C), FFI, HTTP, config, notices, badges, Wi-Fi watcher |
| `crates/slot-cheevos/rcheevos/` | rcheevos source, refreshed with `fetch-rcheevos.sh <tag>` |
| `crates/slot-cheevos/patches/` | fixes applied to rcheevos by `fetch-rcheevos.sh`, in filename order |
| `crates/slot/src/cheevos/` | glue: hooks, login and retry, popup overlay, Wi-Fi watcher thread |
| `crates/slot-retro/src/memory.rs` | captures the core's `SET_MEMORY_MAPS` descriptors |
| `card/System/slotnet.sh`, `card/test-slotnet.sh` | Wi-Fi up/down script and its tests |
| `licenses/rcheevos-MIT.txt`, `licenses/retroachievements-tls.txt` | notices shipped in `System/licenses` |
| `taskfile.fork.yml`, `tools/slotpress.c` | dev-only tasks for a device on adb: screenshots and button presses |

## Hooks in upstream files

`crates/slot/tests/cheevos_hooks.rs` fails if a merge drops any of these.

| File | Hook |
|---|---|
| `taskfile.yml` | `includes:` block that flattens in `taskfile.fork.yml` |
| `crates/slot/Cargo.toml` | `cheevos` feature, on in `default` and `device`; optional `slot-cheevos` dependency |
| `crates/slot/src/lib.rs` | `pub mod cheevos;` |
| `crates/slot/src/frontend.rs` | `crate::cheevos::start(platform.root());` in `boot_with` |
| `crates/slot/src/frontend.rs` | `crate::cheevos::overlay(compositor, &mut self.draws);` in `compose` |
| `crates/slot/src/emu.rs` | `let _cheevos = crate::cheevos::loaded(&rom, resume.is_some());` before the resume state loads |
| `crates/slot/src/emu.rs` | `crate::cheevos::idle(core.as_ref());` after the command drain |
| `crates/slot/src/emu.rs` | `crate::cheevos::frame(core.as_ref());` after every `retro_run` |
| `crates/slot/src/emu.rs` | `crate::cheevos::reset();` after `Cmd::Load` and the cable state restore |
| `crates/slot-retro/src/lib.rs` | `mod memory;` and `pub use memory::MemoryDescriptor;` |
| `crates/slot-retro/src/core.rs` | default `fn memory_map` on `RetroCore` |
| `crates/slot-retro/src/libretro.rs` | `memory_map` field on `Host`, the `SET_MEMORY_MAPS` arm, `fn memory_map` for `LibretroCore` |

Building without the feature gives exactly upstream's behaviour:
`cargo test -p slot --no-default-features --features host`.

## Driving a device over adb

With the device on USB, these let you see and drive slot without touching it:

```sh
task screenshot:device                         # saves target-device/screenshot.png
task press:device -- right right a             # taps
task press:device -- left:600 menu+x wait:500 b   # a hold, a chord, a pause
```

`press:device` builds `tools/slotpress.c` in the device container and pushes it to `/tmp` on the device.
It writes key events into the existing `/dev/input` nodes, which slot reads like real presses. Buttons:
`a b x y l1 r1 l2 r2 start select menu up down left right power volup voldown`. Holding A on the shelf
starts a game fresh, which later overwrites its resume state, so tap A unless that is the point.

## Syncing with upstream

```sh
git fetch origin
git merge origin/main
```

- A conflict in an upstream file is almost always one of the hook lines above. Keep upstream's version and
  put the hook line back next to the same statement.
- For `Cargo.lock`, take upstream's copy and let cargo add this fork's dependencies back:
  `git checkout --theirs Cargo.lock && cargo metadata --format-version 1 > /dev/null`.
- Run `task check`. The hook census test catches a dropped hook.

## Updating rcheevos

```sh
crates/slot-cheevos/fetch-rcheevos.sh v12.6.0
cargo test -p slot-cheevos
```

If a patch no longer applies, check whether upstream rcheevos fixed the bug and delete the patch if so. Both
current patches stop a crash when a game is unloaded right after its last achievement unlocks.
