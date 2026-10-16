use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};

use slot_gfx::Compositor;
use slot_input::{Action, Btn};
use slot_store::collections::{read_lists, CartKey, Index, LISTS_DIR};
use slot_store::{Cart, Platform};
use slot_ui::{Draw, KeyboardFaces, Shelf, TexId, UndoFace};

use super::{App, Phase, Rack, FAVORITES_AT};

mod search;

pub use search::Search;

const SEARCH_AT: usize = FAVORITES_AT + 1;

#[derive(Default)]
pub struct Collections {
    stocked: bool,
    search: Option<Search>,
    search_from: usize,
    last_query: String,
    faces: KeyboardFaces,
    building: Option<Receiver<(Vec<UndoFace>, Vec<UndoFace>)>>,
    faces_for: Option<String>,
    match_slots: Vec<Option<TexId>>,
}

pub fn stock(app: &mut App, root: &Path) {
    let _ = std::fs::create_dir_all(root.join(LISTS_DIR));
    let index = Index::new(app.carts());
    let mut racks = vec![(Rack::Search, Shelf::new(Vec::new()))];
    for list in read_lists(root, &index) {
        let name: &'static str = Box::leak(list.name.into_boxed_str());
        racks.push((Rack::List(name), Shelf::new(carts_for(app, &list.carts))));
    }
    let added = racks.len();
    app.shelves.splice(SEARCH_AT..SEARCH_AT, racks);
    if app.shelf_at >= SEARCH_AT {
        app.shelf_at += added;
    }
    app.collections.stocked = true;
}

fn carts_for(app: &App, keys: &[CartKey]) -> Vec<Cart> {
    let by_key: HashMap<(Platform, &str), &Cart> = app
        .carts()
        .map(|c| ((c.platform, c.stem.as_str()), c))
        .collect();
    keys.iter()
        .filter_map(|(p, stem)| by_key.get(&(*p, stem.as_str())))
        .map(|c| (*c).clone())
        .collect()
}

fn go_to(app: &mut App, to: usize) {
    app.shelf_mut().release_hold();
    app.play_held = None;
    app.shelf_at = to;
    app.shelf_platform = slot_ui::Printed::default();
    app.slot_letter = None;
    app.shelf_named = None;
    app.name_pending = true;
}

pub fn input(app: &mut App, action: Action) -> bool {
    let c = &app.collections;
    if !c.stocked || !matches!(app.phase, Phase::Shelf) || app.core_picker.is_some() {
        return false;
    }
    if c.search.is_some() {
        search::input(app, action);
        return true;
    }
    match action {
        Action::GbaDown(Btn::X) => search::open(app),
        Action::GbaDown(Btn::B) if app.shelf_at == SEARCH_AT => search::clear(app),
        _ => return false,
    }
    true
}

pub fn search_title(app: &App) -> Option<String> {
    let on_search = app.shelves[app.shelf_at].0 == Rack::Search;
    (on_search && app.slot_letter.is_none()).then(|| quoted(&app.collections.last_query))
}

fn quoted(query: &str) -> String {
    format!("\"{query}\"")
}

pub fn draw(app: &App, out: &mut Vec<Draw>) {
    if !matches!(app.phase, Phase::Shelf) {
        return;
    }
    if let Some(search) = &app.collections.search {
        let shake = app.shake_when(true, app.now());
        search
            .keyboard
            .draw(app.canvas(), &app.collections.faces, shake, out);
    }
}

pub fn sync_faces(app: &mut App, compositor: &mut Compositor) {
    let c = &mut app.collections;
    if !c.stocked {
        return;
    }
    if c.faces.keys.is_empty() {
        match &c.building {
            None => {
                let (tx, rx) = mpsc::channel();
                let spawned = std::thread::Builder::new()
                    .name("slot-keyboard".into())
                    .spawn(move || {
                        let _ = tx.send((
                            slot_ui::keyboard_key_faces(),
                            slot_ui::keyboard_legend_faces(),
                        ));
                    });
                if spawned.is_ok() {
                    c.building = Some(rx);
                }
            }
            Some(rx) => {
                if let Ok((keys, legend)) = rx.try_recv() {
                    c.faces.keys = keys
                        .iter()
                        .map(|f| (compositor.create_texture(f.w, f.h, &f.rgba), f.w, f.h))
                        .collect();
                    c.faces.legend = legend
                        .iter()
                        .map(|f| (compositor.create_texture(f.w, f.h, &f.rgba), f.w))
                        .collect();
                }
            }
        }
    }
    let Some(search) = &c.search else {
        c.faces_for = None;
        return;
    };
    if c.faces_for.as_deref() == Some(search.query.as_str()) {
        return;
    }
    c.faces_for = Some(search.query.clone());
    let query = slot_ui::keyboard_query_face(&search.query);
    c.faces.query = Some(put(compositor, c.faces.query.map(|q| q.0), query));
    let count = slot_ui::keyboard_line_face(&search.count_text());
    c.faces.count = Some(put(compositor, c.faces.count.map(|q| q.0), count));
    let lines = search.shown();
    c.match_slots.resize(slot_ui::SHOWN_MATCHES, None);
    c.faces.matches.clear();
    for (slot, line) in c.match_slots.iter_mut().zip(lines) {
        let placed = put(compositor, *slot, slot_ui::keyboard_line_face(&line));
        *slot = Some(placed.0);
        c.faces.matches.push(placed);
    }
}

fn put(compositor: &mut Compositor, slot: Option<TexId>, face: UndoFace) -> (TexId, u32, u32) {
    let tex = match slot {
        Some(tex) => {
            compositor.update_texture(tex, face.w, face.h, &face.rgba);
            tex
        }
        None => compositor.create_texture(face.w, face.h, &face.rgba),
    };
    (tex, face.w, face.h)
}

impl App {
    pub fn shelf_names(&self) -> Vec<String> {
        self.shelves
            .iter()
            .filter(|(_, s)| !s.carts.is_empty())
            .map(|(rack, _)| match rack {
                Rack::Search => quoted(&self.collections.last_query),
                _ => rack.name().to_string(),
            })
            .collect()
    }

    pub fn shelf_stems(&self) -> Vec<String> {
        self.shelf().carts.iter().map(|c| c.stem.clone()).collect()
    }

    pub fn search(&self) -> Option<&Search> {
        self.collections.search.as_ref()
    }
}
