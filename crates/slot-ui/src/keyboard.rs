use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::plate::{centred_hints, hint_face, title_face, word_face, UndoFace, HINT_H, LEGEND_GAP};
use crate::power_menu::menu_face;
use crate::slot_chrome::{edge, opening, recess};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Key {
    Char(char),
    Space,
    Delete,
    Done,
}

pub const COLS: usize = 10;
pub const KEY_W: f32 = 60.0;
pub const KEY_H: f32 = 44.0;
pub const KEY_GAP: f32 = 6.0;
pub const KEYS_X: f32 = (OUT_W as f32 - (COLS as f32 * (KEY_W + KEY_GAP) - KEY_GAP)) / 2.0;
pub const KEYS_Y: f32 = 204.0;
pub const FIELD_Y: f32 = 26.0;
pub const FIELD_H: f32 = 48.0;
const FIELD_W: f32 = 560.0;
const COUNT_Y: f32 = 82.0;
const MATCH_Y: f32 = 112.0;
const MATCH_PITCH: f32 = 28.0;
pub const SHOWN_MATCHES: usize = 3;
pub const KEYBOARD_LEGEND_Y: f32 = 427.0;

const ROWS: [&str; 4] = ["1234567890", "QWERTYUIOP", "ASDFGHJKL<", "ZXCVBNM__>"];

fn key_at(row: usize, col: usize) -> Key {
    match ROWS[row].as_bytes()[col] {
        b'<' => Key::Delete,
        b'_' => Key::Space,
        b'>' => Key::Done,
        c => Key::Char(c as char),
    }
}

fn span(row: usize, col: usize) -> (usize, usize) {
    let key = key_at(row, col);
    let mut start = col;
    while start > 0 && key_at(row, start - 1) == key {
        start -= 1;
    }
    let mut end = col;
    while end + 1 < COLS && key_at(row, end + 1) == key {
        end += 1;
    }
    (start, end)
}

pub fn all_keys() -> Vec<Key> {
    let mut keys = Vec::new();
    for row in 0..ROWS.len() {
        for col in 0..COLS {
            let key = key_at(row, col);
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    keys
}

pub fn key_text(key: Key) -> String {
    match key {
        Key::Char(c) => c.to_string(),
        Key::Space => "Space".into(),
        Key::Delete => "Del".into(),
        Key::Done => "OK".into(),
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Keyboard {
    row: usize,
    col: usize,
    home: usize,
}

impl Default for Keyboard {
    fn default() -> Self {
        Keyboard {
            row: 1,
            col: 0,
            home: 0,
        }
    }
}

impl Keyboard {
    pub fn key(&self) -> Key {
        key_at(self.row, self.col)
    }

    pub fn at(&self) -> (usize, usize) {
        (self.row, span(self.row, self.col).0)
    }

    pub fn up(&mut self) {
        self.row = (self.row + ROWS.len() - 1) % ROWS.len();
        self.col = self.home;
    }

    pub fn down(&mut self) {
        self.row = (self.row + 1) % ROWS.len();
        self.col = self.home;
    }

    pub fn left(&mut self) {
        let (start, _) = span(self.row, self.col);
        let col = (start + COLS - 1) % COLS;
        self.col = span(self.row, col).0;
        self.home = self.col;
    }

    pub fn right(&mut self) {
        let (_, end) = span(self.row, self.col);
        self.col = (end + 1) % COLS;
        self.home = self.col;
    }

    pub fn key_rect(row: usize, col: usize) -> (f32, f32, f32, f32) {
        let (start, end) = span(row, col);
        let wide = (end - start + 1) as f32;
        (
            KEYS_X + start as f32 * (KEY_W + KEY_GAP),
            KEYS_Y + row as f32 * (KEY_H + KEY_GAP),
            wide * KEY_W + (wide - 1.0) * KEY_GAP,
            KEY_H,
        )
    }
}

pub fn keyboard_key_faces() -> Vec<UndoFace> {
    all_keys()
        .into_iter()
        .map(|k| match k {
            Key::Char(_) => menu_face(&key_text(k)),
            _ => word_face(&key_text(k)),
        })
        .collect()
}

pub fn keyboard_legend_faces() -> Vec<UndoFace> {
    vec![
        hint_face("A", "Type"),
        hint_face("B", "Delete"),
        hint_face("Y", "Space"),
        hint_face("START", "Search"),
    ]
}

pub fn keyboard_query_face(query: &str) -> UndoFace {
    menu_face(&format!("{query}_"))
}

pub fn keyboard_line_face(text: &str) -> UndoFace {
    title_face(text)
}

#[derive(Default)]
pub struct KeyboardFaces {
    pub keys: Vec<(TexId, u32, u32)>,
    pub legend: Vec<(TexId, u32)>,
    pub query: Option<(TexId, u32, u32)>,
    pub count: Option<(TexId, u32, u32)>,
    pub matches: Vec<(TexId, u32, u32)>,
}

impl Keyboard {
    pub fn draw(&self, faces: &KeyboardFaces, shake: f32, out: &mut Vec<Draw>) {
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: OUT_W as f32,
            h: OUT_H as f32,
            colour: opening(),
        });
        out.push(Draw::Rect {
            x: ((OUT_W as f32 - FIELD_W) / 2.0 + shake).round(),
            y: FIELD_Y,
            w: FIELD_W,
            h: FIELD_H,
            colour: recess(),
        });
        if let Some((tex, w, h)) = faces.query {
            let y = FIELD_Y + (FIELD_H - h as f32) / 2.0;
            push(
                out,
                tex,
                (OUT_W as f32 - w as f32) / 2.0 + shake,
                y,
                w as f32,
                h,
            );
        }
        if let Some((tex, w, h)) = faces.count {
            push(
                out,
                tex,
                (OUT_W as f32 - w as f32) / 2.0,
                COUNT_Y,
                w as f32,
                h,
            );
        }
        for (i, &(tex, w, h)) in faces.matches.iter().take(SHOWN_MATCHES).enumerate() {
            let y = MATCH_Y + i as f32 * MATCH_PITCH;
            push(out, tex, (OUT_W as f32 - w as f32) / 2.0, y, w as f32, h);
        }
        let lit = self.at();
        let keys = all_keys();
        for row in 0..ROWS.len() {
            let mut col = 0;
            while col < COLS {
                let (x, y, w, h) = Keyboard::key_rect(row, col);
                let colour = if (row, col) == lit { edge() } else { recess() };
                out.push(Draw::Rect { x, y, w, h, colour });
                let key = key_at(row, col);
                let face = keys
                    .iter()
                    .position(|k| *k == key)
                    .and_then(|i| faces.keys.get(i));
                if let Some(&(tex, fw, fh)) = face {
                    let fw = fw as f32;
                    push(
                        out,
                        tex,
                        x + (w - fw) / 2.0,
                        y + (h - fh as f32) / 2.0,
                        fw,
                        fh,
                    );
                }
                col = span(row, col).1 + 1;
            }
        }
        for (tex, w, x) in centred_hints(&faces.legend, LEGEND_GAP) {
            push(out, tex, x, KEYBOARD_LEGEND_Y, w as f32, HINT_H);
        }
    }
}

fn push(out: &mut Vec<Draw>, tex: TexId, x: f32, y: f32, w: f32, h: u32) {
    out.push(Draw::Tex {
        x: x.round(),
        y: y.round(),
        w,
        h: h as f32,
        tex,
        alpha: 1.0,
    });
}
