use slot_store::{Cart, Platform};
use slot_ui::{Draw, Shelf, TexId, CART_W, OUT_W};

fn cart(platform: Platform, stem: &str) -> Cart {
    Cart {
        platform,
        stem: stem.to_string(),
        rom: format!("Games/{}/{stem}", platform.dir_name()).into(),
        label: None,
        code: String::new(),
        shell: None,
        title: String::new(),
    }
}

fn gba(stems: &[&str]) -> Vec<Cart> {
    stems.iter().map(|s| cart(Platform::Gba, s)).collect()
}

fn faced(carts: Vec<Cart>) -> Shelf {
    let n = carts.len();
    let mut shelf = Shelf::new(carts);
    shelf.set_faces((0..n).map(TexId::from_raw).collect());
    shelf
}

fn stems(shelf: &Shelf) -> Vec<&str> {
    shelf.carts.iter().map(|c| c.stem.as_str()).collect()
}

fn centres(shelf: &Shelf) -> Vec<f32> {
    let mut out = Vec::new();
    shelf.draw_row(None, 0.0, 0.0, 1.0, &mut out);
    let mut xs: Vec<f32> = out
        .iter()
        .filter_map(|d| match *d {
            Draw::Rect { x, w, .. } | Draw::Tex { x, w, .. } => Some(x + w / 2.0),
            _ => None,
        })
        .collect();
    xs.sort_by(f32::total_cmp);
    xs.dedup();
    xs
}

#[test]
fn restocking_keeps_each_carts_face_and_hands_back_the_rest() {
    let mut shelf = faced(gba(&["A", "B", "C"]));
    let spare = shelf.restock(gba(&["C", "D", "A"]), None);
    assert_eq!(shelf.face(0), Some(TexId::from_raw(2)));
    assert_eq!(
        shelf.face(1),
        None,
        "a new cart borrowed someone else's face"
    );
    assert_eq!(shelf.face(2), Some(TexId::from_raw(0)));
    assert_eq!(spare, vec![TexId::from_raw(1)]);
}

#[test]
fn restocking_keeps_the_selected_cart_under_the_slot() {
    let mut shelf = faced(gba(&["A", "B", "C", "D"]));
    shelf.select(2);
    let keep = shelf.carts[2].rom.clone();
    shelf.restock(gba(&["Z", "A", "B", "C", "D"]), Some(&keep));
    assert_eq!(shelf.index, 3);
    assert_eq!(stems(&shelf)[shelf.index], "C");
    assert_eq!(
        shelf.scroll,
        shelf.scroll_target(),
        "the row moved under the player"
    );
}

#[test]
fn restocking_without_the_selected_cart_slides_its_neighbour_in() {
    let mut shelf = faced(gba(&["A", "B", "C"]));
    shelf.select(1);
    let keep = shelf.carts[1].rom.clone();
    shelf.restock(gba(&["A", "C"]), Some(&keep));
    assert_eq!(stems(&shelf)[shelf.index], "C");

    shelf.select(1);
    let keep = shelf.carts[1].rom.clone();
    shelf.restock(gba(&["A"]), Some(&keep));
    assert_eq!(
        shelf.index, 0,
        "the last cart going left the index past the end"
    );

    shelf.restock(Vec::new(), None);
    assert_eq!(shelf.index, 0);
    assert!(shelf.carts.is_empty());
}

#[test]
fn a_shelf_of_mixed_platforms_spaces_its_carts_evenly() {
    let mut shelf = Shelf::new(vec![
        cart(Platform::Gba, "A"),
        cart(Platform::Gb, "B"),
        cart(Platform::Gbc, "C"),
        cart(Platform::Gba, "D"),
        cart(Platform::Gb, "E"),
    ]);
    shelf.select(2);
    let xs = centres(&shelf);
    let mid = OUT_W as f32 / 2.0;
    assert!(xs.iter().any(|x| (x - mid).abs() < 0.5), "{xs:?}");
    for pair in xs.windows(2) {
        assert!(
            (pair[1] - pair[0] - CART_W as f32).abs() < 0.5,
            "carts sat {} apart instead of a cart's width: {xs:?}",
            pair[1] - pair[0]
        );
    }
}

#[test]
fn a_shelf_of_one_platform_is_spaced_as_before() {
    let mut gb = Shelf::new(
        ["A", "B", "C"]
            .iter()
            .map(|s| cart(Platform::Gb, s))
            .collect(),
    );
    gb.select(1);
    let xs = centres(&gb);
    let pitch = slot_ui::cart_box(Platform::Gb).0 as f32;
    for pair in xs.windows(2) {
        assert!((pair[1] - pair[0] - pitch).abs() < 0.5, "{xs:?}");
    }
}
