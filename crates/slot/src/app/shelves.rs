use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use slot_gfx::Compositor;
use slot_input::{Action, Btn};
use slot_store::collections::{
    hits, read_entries, read_lists, render, toggle, CartKey, Entry, Index, FAVORITES_FILE,
    LISTS_DIR, RECENT_FILE,
};
use slot_store::{sort_key, Cart, Platform};
use slot_ui::{Draw, KeyboardFaces, Shelf, TexId, UndoFace};

use super::{App, Phase};

mod search;
mod writer;

pub use search::Search;
use writer::ListWriter;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ShelfKind {
    Platform(Platform),
    Recent,
    Favorites,
    List(String),
    Search(String),
}

impl ShelfKind {
    pub fn platform(&self) -> Option<Platform> {
        match self {
            ShelfKind::Platform(p) => Some(*p),
            _ => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ShelfKind::Platform(p) => p.name(),
            ShelfKind::Recent => "Recently Played",
            ShelfKind::Favorites => "Favorites",
            ShelfKind::List(name) | ShelfKind::Search(name) => name,
        }
    }
}

impl From<Platform> for ShelfKind {
    fn from(p: Platform) -> Self {
        ShelfKind::Platform(p)
    }
}

const SEARCH_AT: usize = 0;
const RECENT_AT: usize = 1;
const FAVORITES_AT: usize = 2;

#[derive(Default)]
pub struct Collections {
    root: Option<PathBuf>,
    recent: Vec<Entry>,
    favorites: Vec<Entry>,
    recent_dirty: bool,
    note: Option<&'static str>,
    shed: Vec<TexId>,
    writer: Option<ListWriter>,
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
    let recent = read_entries(&root.join(RECENT_FILE), &index);
    let favorites = read_entries(&root.join(FAVORITES_FILE), &index);
    let mut front = vec![
        (ShelfKind::Search(String::new()), Shelf::new(Vec::new())),
        (
            ShelfKind::Recent,
            Shelf::new(carts_for(app, &hits(&recent))),
        ),
        (
            ShelfKind::Favorites,
            Shelf::new(favourite_carts(app, &favorites)),
        ),
    ];
    for list in read_lists(root, &index) {
        let carts = carts_for(app, &list.carts);
        front.push((ShelfKind::List(list.name), Shelf::new(carts)));
    }
    let added = front.len();
    app.shelves.splice(0..0, front);
    app.shelf_at = match app.shelves[RECENT_AT].1.carts.is_empty() {
        true => app.shelf_at + added,
        false => RECENT_AT,
    };
    app.collections = Collections {
        root: Some(root.to_path_buf()),
        recent,
        favorites,
        writer: Some(ListWriter::spawn(app.card_failing.clone())),
        ..Collections::default()
    };
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

fn favourite_carts(app: &App, favorites: &[Entry]) -> Vec<Cart> {
    let mut carts = carts_for(app, &hits(favorites));
    carts.sort_by(|a, b| {
        (sort_key(&a.stem), a.platform as u8).cmp(&(sort_key(&b.stem), b.platform as u8))
    });
    carts
}

fn stocked(app: &App) -> bool {
    app.collections.root.is_some()
}

fn save(app: &App, file: &str, entries: &[Entry]) {
    let (Some(root), Some(writer)) = (&app.collections.root, &app.collections.writer) else {
        return;
    };
    writer.save(root.join(file), render(entries));
}

fn restock(app: &mut App, at: usize, carts: Vec<Cart>) {
    let shelf = &mut app.shelves[at].1;
    let keep = shelf.carts.get(shelf.index).map(|c| c.rom.clone());
    let spare = shelf.restock(carts, keep.as_deref());
    app.collections.shed.extend(spare);
}

fn restock_recent(app: &mut App) {
    let carts = carts_for(app, &hits(&app.collections.recent));
    restock(app, RECENT_AT, carts);
    app.shelves[RECENT_AT].1.select(0);
    app.collections.recent_dirty = false;
}

fn go_to(app: &mut App, to: usize) {
    if to == app.shelf_at {
        return;
    }
    app.shelf_mut().release_hold();
    app.play_held = None;
    app.shelf_at = to;
    app.shelf_platform = slot_ui::Printed::default();
    app.slot_letter = None;
    app.shelf_named = None;
    app.name_pending = true;
}

fn leave_recent(app: &mut App) {
    if app.shelf_at == RECENT_AT && app.collections.recent_dirty {
        restock_recent(app);
    }
}

pub fn input(app: &mut App, action: Action) -> bool {
    if !stocked(app) || !matches!(app.phase, Phase::Shelf) || app.core_picker.is_some() {
        return false;
    }
    if app.collections.search.is_some() {
        search::input(app, action);
        return true;
    }
    match action {
        Action::GbaDown(Btn::Y) => favourite(app),
        Action::GbaDown(Btn::X) => search::open(app),
        Action::GbaDown(Btn::B) if app.shelf_at == SEARCH_AT => search::clear(app),
        Action::GbaDown(btn @ (Btn::L1 | Btn::R1)) => {
            app.collections.note = None;
            let by = if btn == Btn::L1 { -1 } else { 1 };
            if app.next_shelf(by).is_some() {
                leave_recent(app);
            }
            return false;
        }
        Action::GbaDown(Btn::Up | Btn::Down) => {
            app.collections.note = None;
            return false;
        }
        _ => return false,
    }
    true
}

fn favourite(app: &mut App) {
    let Some(cart) = app.shelf().carts.get(app.shelf().index).cloned() else {
        return;
    };
    let now = toggle(&mut app.collections.favorites, cart.platform, &cart.stem);
    save(app, FAVORITES_FILE, &app.collections.favorites);
    let carts = favourite_carts(app, &app.collections.favorites);
    restock(app, FAVORITES_AT, carts);
    if app.shelf_at == FAVORITES_AT && app.shelf().carts.is_empty() {
        if let Some(to) = app.next_shelf(1) {
            go_to(app, to);
        }
    }
    note(app, if now { "Favorited" } else { "Unfavorited" });
}

fn note(app: &mut App, text: &'static str) {
    if app.slot_text().as_deref() != Some(text) {
        app.shelf_platform = slot_ui::Printed::default();
    }
    app.collections.note = Some(text);
    app.shelf_named = None;
    app.name_pending = true;
}

pub fn shown_note(app: &App) -> Option<&'static str> {
    app.collections.note
}

pub fn recorded(app: &mut App, cart: Option<&str>) {
    if !stocked(app) || cart.is_none() {
        return;
    }
    let Some(seated) = app.seated_cart() else {
        return;
    };
    let (platform, stem) = (seated.platform, seated.stem.clone());
    if !slot_store::collections::played(&mut app.collections.recent, platform, &stem) {
        return;
    }
    save(app, RECENT_FILE, &app.collections.recent);
    match app.shelf_at == RECENT_AT {
        true => app.collections.recent_dirty = true,
        false => restock_recent(app),
    }
}

pub fn take_shed(app: &mut App) -> Vec<TexId> {
    std::mem::take(&mut app.collections.shed)
}

pub fn draw(app: &App, out: &mut Vec<Draw>) {
    if !matches!(app.phase, Phase::Shelf) {
        return;
    }
    if let Some(search) = &app.collections.search {
        let shake = app.shake_when(true, app.now());
        search.keyboard.draw(&app.collections.faces, shake, out);
    }
}

pub fn sync_faces(app: &mut App, compositor: &mut Compositor) {
    let c = &mut app.collections;
    if c.root.is_none() {
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
    pub fn flush_lists(&self) {
        if let Some(writer) = &self.collections.writer {
            writer.flush();
        }
    }

    pub fn shelf_kind(&self) -> &ShelfKind {
        &self.shelves[self.shelf_at].0
    }

    pub fn shelf_kinds(&self) -> Vec<ShelfKind> {
        self.shelves
            .iter()
            .filter(|(_, s)| !s.carts.is_empty())
            .map(|(k, _)| k.clone())
            .collect()
    }

    pub fn shelf_stems(&self) -> Vec<String> {
        self.shelf().carts.iter().map(|c| c.stem.clone()).collect()
    }

    pub fn search(&self) -> Option<&Search> {
        self.collections.search.as_ref()
    }
}
