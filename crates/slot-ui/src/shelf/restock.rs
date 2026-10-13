use std::collections::HashMap;
use std::path::{Path, PathBuf};

use slot_gfx::TexId;
use slot_store::Cart;

use super::Shelf;
use crate::cart::{gb_shell_of, CART_W};

pub(super) fn pitch_of(carts: &[Cart]) -> Option<f32> {
    let first = carts.first()?.platform;
    carts
        .iter()
        .any(|c| c.platform != first)
        .then_some(CART_W as f32)
}

impl Shelf {
    pub fn restock(&mut self, carts: Vec<Cart>, keep: Option<&Path>) -> Vec<TexId> {
        let mut old: HashMap<PathBuf, TexId> = HashMap::new();
        let mut spare = Vec::new();
        for (i, cart) in self.carts.iter().enumerate() {
            if let Some(tex) = self.faces.get_mut(i).and_then(Option::take) {
                if let Some(twin) = old.insert(cart.rom.clone(), tex) {
                    spare.push(twin);
                }
            }
        }
        let in_flight = self.scroll - self.scroll_target();
        let index = match keep.and_then(|rom| carts.iter().position(|c| c.rom == rom)) {
            Some(i) => i,
            None => self.index.min(carts.len().saturating_sub(1)),
        };
        self.faces = carts.iter().map(|c| old.remove(&c.rom)).collect();
        self.shells = carts.iter().map(gb_shell_of).collect();
        self.pitch = pitch_of(&carts);
        self.carts = carts;
        self.held = None;
        self.index = index;
        self.ride = index as f32;
        self.scroll = index as f32 + in_flight;
        spare.extend(old.into_values());
        spare
    }
}
