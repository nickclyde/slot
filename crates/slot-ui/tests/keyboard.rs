use slot_ui::{Canvas, Draw, Key, Keyboard, KeyboardFaces, TexId, HINT_H, OUT_H};

fn walk(moves: &str) -> Keyboard {
    let mut k = Keyboard::default();
    for m in moves.chars() {
        match m {
            'u' => k.up(),
            'd' => k.down(),
            'l' => k.left(),
            'r' => k.right(),
            _ => unreachable!(),
        }
    }
    k
}

#[test]
fn the_keyboard_opens_on_the_first_letter_row() {
    assert_eq!(Keyboard::default().key(), Key::Char('Q'));
}

#[test]
fn the_cursor_wraps_around_every_edge_of_the_keyboard() {
    assert_eq!(walk("l").key(), Key::Char('P'));
    assert_eq!(walk("r").key(), Key::Char('W'));
    assert_eq!(walk("u").key(), Key::Char('1'));
    assert_eq!(walk("uu").key(), Key::Char('Z'));
    assert_eq!(walk("ddd").key(), Key::Char('1'));
    assert_eq!(walk("ddl").key(), Key::Done);
    assert_eq!(walk("ddlr").key(), Key::Char('Z'));
}

#[test]
fn the_wide_keys_are_one_stop_each() {
    assert_eq!(walk("dl").key(), Key::Delete);
    assert_eq!(walk("ddll").key(), Key::Space);
    assert_eq!(walk("ddlll").key(), Key::Char('M'));
}

#[test]
fn leaving_a_wide_key_returns_to_the_column_you_came_from() {
    let mut k = walk("rrrrrrrr");
    assert_eq!(k.key(), Key::Char('O'));
    k.down();
    assert_eq!(k.key(), Key::Char('L'));
    k.down();
    assert_eq!(k.key(), Key::Space);
    k.down();
    assert_eq!(
        k.key(),
        Key::Char('9'),
        "going on through Space lost the column"
    );
}

const FOUR_THREE: Canvas = Canvas {
    w: 640.0,
    h: OUT_H as f32,
};

#[test]
fn the_keyboard_fits_the_screen_above_its_legend() {
    let faces = KeyboardFaces {
        legend: vec![(TexId::from_raw(1), 80)],
        ..KeyboardFaces::default()
    };
    for canvas in [Canvas::FRAMED, FOUR_THREE] {
        let mut out = Vec::new();
        Keyboard::default().draw(canvas, &faces, 0.0, &mut out);
        let legend_top = out
            .iter()
            .find_map(|d| match *d {
                Draw::Tex { y, h, .. } if h == HINT_H as f32 => Some(y),
                _ => None,
            })
            .expect("no legend drawn");
        assert!(legend_top + HINT_H as f32 <= OUT_H as f32);
        for d in &out[1..] {
            if let Draw::Rect { x, y, w, h, .. } = *d {
                assert!(x >= 0.0 && x + w <= canvas.w, "a key ran off the side");
                assert!(y + h <= legend_top, "a key ran into the legend");
            }
        }
    }
}

#[test]
fn only_the_key_under_the_cursor_is_lit() {
    let mut out = Vec::new();
    walk("dl").draw(Canvas::FRAMED, &KeyboardFaces::default(), 0.0, &mut out);
    let lit = slot_ui::edge();
    let lit: Vec<(f32, f32)> = out
        .iter()
        .filter_map(|d| match *d {
            Draw::Rect { x, w, colour, .. } if colour == lit => Some((x, w)),
            _ => None,
        })
        .collect();
    assert_eq!(
        lit,
        vec![Keyboard::key_rect(Canvas::FRAMED, 2, 9)]
            .into_iter()
            .map(|(x, _, w, _)| (x, w))
            .collect::<Vec<_>>()
    );
}
