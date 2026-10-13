# Handoff: RetroAchievements fork

Notes for whoever picks this fork up next. `FORK.md` lists every hook in upstream files and how to sync;
this file covers status, decisions and what is still open. Last updated 2026-10-13.

## Status

- RetroAchievements (softcore) and the Wi-Fi client are implemented, tested and committed on `main` of
  `nickclyde/slot`, based on upstream `4a4ba92` (v1.6.0 plus run-ahead, "Add a 1 frame run-ahead setting
  for GBA games on mGBA").
- Confirmed on a real RG SP: sign-in worked for user `clydedroid`, and inserting Metroid - Zero Mission showed
  the game-loaded banner with 2 achievements. Not yet checked from `slot.log` whether Wi-Fi came up through
  `slotnet.sh` or BaseOS was already online.
- The card used was a 64 GB microSD in SD2, BaseOS in SD1, set up by hand (see "Making a card").

## Tests still to run on the device

1. Earn an achievement live: popup with badge, and it shows as softcore on retroachievements.org.
2. `Config/retroachievements.txt` now has `token = ...` and no password.
3. Eject right after an unlock, then reinsert (the rcheevos crash path, see below).
4. Load a Polaroid taken past a trigger: nothing should unlock by itself.
5. Fast-forward and rewind for a while: no stutter, achievements still trigger.
6. A GB or GBC game with achievements (Link's Awakening DX, Pokemon Red): different memory map.
7. A game set to gpSP: memory map only verified on the host so far.
8. Sleep and wake: Wi-Fi and achievements recover.
9. Go offline mid-game, unlock, come back: Offline then Online banners, unlock reaches the site.
10. Link play: `slot.log` shows `slot: wifi: yielding to link play`, then `slot: wifi: up`.
11. First boot of a cart with `BIOS/gba_bios.bin` present (delete its state ring first): the game-loaded
    banner appears top left only after the BIOS intro, and not too long after.

## How it fits together

- `crates/slot-cheevos`: rcheevos v12.5.0 vendored as C source and built with `cc`, a C shim
  (`csrc/slot_shim.c`) so Rust never mirrors rcheevos struct layouts, the HTTP transport (ureq 2, rustls,
  bundled webpki roots), config parsing, notices, badge decoding and the Wi-Fi watcher state machine.
- `crates/slot/src/cheevos/`: the glue. `live.rs` holds the hooks, the sign-in thread, the face thread and the
  overlay; `off.rs` is the no-op build when the `cheevos` feature is off; `overlay.rs` draws the 480x84 banner;
  `net.rs` runs `System/slotnet.sh` on the device.
- `crates/slot-retro/src/memory.rs`: captures the core's `SET_MEMORY_MAPS` descriptors.
- Upstream files only carry one-line hooks. `crates/slot/tests/cheevos_hooks.rs` fails if a merge drops one.

## Decisions and why

- **Softcore only.** slot resumes from a save state on every insert, and hardcore forbids loading states,
  rewind and slow motion. Award requests are asserted to carry `h=0`.
- **One leaked `rc_client` for the whole process.** HTTP callbacks can be in flight at any time, so the
  client is never destroyed. Per-game state (memory regions, console, ticket) lives on the emulator thread.
- **Memory regions are built lazily.** mGBA only declares its memory map on the first `retro_run`, not in
  `retro_load_game`. The emulator thread makes no `rc_client` call until regions exist, because game
  activation reads memory and would otherwise disable achievements as invalid.
- **Background memory reads are off.** Activation and reads happen in `rc_client_do_frame` or
  `rc_client_idle` on the emulator thread only.
- **Tickets guard game changes.** A link reload builds the new emulator before dropping the old one, so the
  old thread's guard only unloads the game if its ticket is still current.
- **Reset policy.** `rc_client_reset` runs after Polaroid loads, undo and the cable state restore, so a loaded
  state cannot trigger an unlock by itself. Rewind pops do not reset, so in-progress hit counts survive.
- **Banners wait out the BIOS intro.** On a fresh boot (no resume state) with a boot ROM in `BIOS/`, the
  overlay pops no new banner for `boot::FRAMES` (300) emulated frames, so the game-loaded banner never covers
  the logo. Counting frames, not time, keeps menus and fast-forward honest. The length is a generous guess;
  tune it on the device if the banner lands too late or still overlaps the intro.
- **`do_frame` on every sub-frame**, fast-forward included, because game memory advances each one.
- **Run-ahead is invisible to achievements.** Upstream's run-ahead runs speculative frames and loads the
  real state back before `crate::cheevos::frame` runs, so `do_frame` only ever sees real memory, once per frame.
- **Vendored rcheevos, not a submodule.** A fresh clone builds with no network, and the release workflow
  (an upstream file) would need editing to fetch submodules.
- **Changelog lives in `FORK.md`**, not `CHANGELOG.md`, whose top changes every upstream release.

## Gotchas found along the way

- **rcheevos crashes when a game unloads right after its last achievement unlocks.** Two null dereferences
  of `client->game`: in `rc_client_raise_mastery_event` and in `rc_client_award_achievement_callback`. Both
  are patched in `crates/slot-cheevos/patches/` and covered by tests that segfault without them. Worth
  reporting upstream to RetroAchievements; drop the patches once fixed there.
- **`SET_MEMORY_MAPS` is `36 | 0x10000`** (the experimental flag). Both values are accepted.
- **`a_stalled_display_does_not_stall_the_worker`** in `crates/slot/tests/emu.rs` is timing sensitive. It fails
  on untouched upstream too when the Mac is busy; rerun it on a quiet machine before suspecting a change.
- **Upstream fails `task check` on its own (v1.6.0 through `4a4ba92`).** `crates/slot/tests/prefetch.rs` is
  not rustfmt-clean and has an unused `let mut step`, so `fmt:check` and `lint` fail. Leave it for upstream to
  fix rather than editing the file and creating merge conflicts; run `lint:device`, `test` and `test:card` on their own.
- **Do not share `target/` between this checkout and a worktree of upstream.** Cargo names workspace crate
  artifacts relative to the workspace root, so one build overwrites the other's and stale artifacts look
  fresh. `cargo clean -p <crate>` for each workspace crate fixes it.
- **In POSIX `sh`, `VAR=x some_function` leaks `VAR`** past the call. The card tests scope overrides with
  `(export VAR=x; run ...)`.
- BaseOS's DHCP client was not known at build time. `slotnet.sh` tries `udhcpc`, then `busybox udhcpc`, and
  exits 5 if neither exists; static `address`, `gateway` and `dns` in `wifi.txt` skip DHCP.

## Making a card

```sh
task dist:device
diskutil list                                          # find the card, e.g. "Built In SDXC Reader"
diskutil eraseDisk FAT32 SLOT MBRFormat /dev/diskN     # whole card, MBR, FAT32
cp -R dist-device/. /Volumes/SLOT/
cp ~/ROMs/GBA/*.gba /Volumes/SLOT/Games/GBA/
dot_clean -m /Volumes/SLOT
diskutil eject /dev/diskN
```

Disk Utility's Erase only reformats the selected volume. Use View, Show All Devices and erase the reader
itself, or the shell will keep an old partition size.

Moving a save from RetroArch: copy the `.srm` to `Saves/GBA/<ROM name>.sav`. It is the same raw save memory.
Save states from other frontends do not carry over.

## Ideas not done

- Keep in-progress achievement progress across Polaroid loads and restarts by storing
  `rc_client_serialize_progress` next to each state in the ring.
- A screen to browse a game's achievements, and a quick-menu entry to sign out or turn achievements off.
- Prebuilt releases for the fork from a GitHub workflow (a new workflow file, so it never conflicts).
- Register a client name with RetroAchievements; not needed for softcore.
- Offer the memory-map capture in `slot-retro` upstream, which would shrink the fork's hooks.
