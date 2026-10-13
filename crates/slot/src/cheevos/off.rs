use std::path::Path;

use slot_gfx::{Compositor, Draw};
use slot_retro::RetroCore;

pub struct Guard;

pub fn start(_root: &Path) {}

pub fn loaded(_rom: &Path, _resumed: bool) -> Guard {
    Guard
}

pub fn idle(_core: &dyn RetroCore) {}

pub fn frame(_core: &dyn RetroCore) {}

pub fn reset() {}

pub fn overlay(_compositor: &mut Compositor, _draws: &mut Vec<Draw>) {}
