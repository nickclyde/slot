use slot_input::{Action, Btn};
use slot_store::find::{fold, matches, Folded};
use slot_store::{sort_key, Cart};
use slot_ui::{clean_label, Key, Keyboard, SHOWN_MATCHES};

use super::{go_to, App, SEARCH_AT};

const QUERY_MAX: usize = 24;

pub struct Search {
    pub query: String,
    pub keyboard: Keyboard,
    pool: Vec<Cart>,
    titles: Vec<Folded>,
    hits: Vec<usize>,
}

impl Search {
    fn new(mut pool: Vec<Cart>, query: String) -> Self {
        pool.sort_by(|a, b| {
            (sort_key(&a.stem), a.platform as u8).cmp(&(sort_key(&b.stem), b.platform as u8))
        });
        let titles = pool.iter().map(|c| fold(&c.stem)).collect();
        let mut search = Search {
            query,
            keyboard: Keyboard::default(),
            pool,
            titles,
            hits: Vec::new(),
        };
        search.refresh();
        search
    }

    fn refresh(&mut self) {
        self.hits = matches(&self.titles, &fold(&self.query));
    }

    pub fn stems(&self) -> Vec<&str> {
        self.hits
            .iter()
            .map(|i| self.pool[*i].stem.as_str())
            .collect()
    }

    pub fn count_text(&self) -> String {
        match self.hits.len() {
            _ if fold(&self.query).is_empty() => String::new(),
            0 => "No games".into(),
            1 => "1 game".into(),
            n => format!("{n} games"),
        }
    }

    pub fn shown(&self) -> Vec<String> {
        self.hits
            .iter()
            .take(SHOWN_MATCHES)
            .map(|i| clean_label(&self.pool[*i].stem))
            .collect()
    }

    fn carts(&self) -> Vec<Cart> {
        self.hits.iter().map(|i| self.pool[*i].clone()).collect()
    }

    fn type_text(&mut self, text: &str) {
        if self.query.chars().count() < QUERY_MAX {
            self.query.push_str(text);
            self.refresh();
        }
    }

    fn delete(&mut self) -> bool {
        let deleted = self.query.pop().is_some();
        self.refresh();
        deleted
    }
}

pub(super) fn open(app: &mut App) {
    let query = match app.shelf_at == SEARCH_AT {
        true => app.collections.last_query.clone(),
        false => String::new(),
    };
    let pool = app.carts().cloned().collect();
    app.shelf_mut().release_hold();
    app.play_held = None;
    app.collections.search = Some(Search::new(pool, query));
}

fn close(app: &mut App) {
    app.collections.search = None;
}

pub(super) fn input(app: &mut App, action: Action) {
    let Some(search) = app.collections.search.as_mut() else {
        return;
    };
    match action {
        Action::GbaDown(Btn::Up) => search.keyboard.up(),
        Action::GbaDown(Btn::Down) => search.keyboard.down(),
        Action::GbaDown(Btn::Left) => search.keyboard.left(),
        Action::GbaDown(Btn::Right) => search.keyboard.right(),
        Action::GbaDown(Btn::A) => match search.keyboard.key() {
            Key::Char(c) => search.type_text(&c.to_string()),
            Key::Space => search.type_text(" "),
            Key::Delete => {
                search.delete();
            }
            Key::Done => confirm(app),
        },
        Action::GbaDown(Btn::Y | Btn::R1) => search.type_text(" "),
        Action::GbaDown(Btn::L1) => {
            search.delete();
        }
        Action::GbaDown(Btn::B) => {
            if !search.delete() {
                close(app);
            }
        }
        Action::GbaDown(Btn::Start) => confirm(app),
        Action::GbaDown(Btn::X) | Action::QuickMenu => close(app),
        _ => {}
    }
}

fn confirm(app: &mut App) {
    let Some(search) = &app.collections.search else {
        return;
    };
    if search.hits.is_empty() {
        app.refuse();
        return;
    }
    let carts = search.carts();
    let query = search.query.trim().to_string();
    if app.shelf_at != SEARCH_AT {
        app.collections.search_from = app.shelf_at;
    }
    app.restock(SEARCH_AT, carts);
    app.shelves[SEARCH_AT].1.select(0);
    app.mark_stars();
    app.collections.last_query = query;
    close(app);
    go_to(app, SEARCH_AT);
}

pub(super) fn clear(app: &mut App) {
    let back = app.collections.search_from;
    let to = Some(back)
        .filter(|at| !app.shelves[*at].1.carts.is_empty())
        .or_else(|| app.next_shelf(1));
    if let Some(to) = to {
        go_to(app, to);
    }
    app.restock(SEARCH_AT, Vec::new());
    app.collections.last_query.clear();
}
