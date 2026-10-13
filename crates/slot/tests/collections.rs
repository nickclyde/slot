mod common;

use std::path::Path;

use slot::app::shelves::ShelfKind;
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, Platform, SlotState};
use tempfile::TempDir;

fn card(gba: &[&str], gb: &[&str], gbc: &[&str]) -> TempDir {
    let d = common::tmp_root_with_carts(gba);
    for (platform, stems) in [(Platform::Gb, gb), (Platform::Gbc, gbc)] {
        for stem in stems {
            let rom = d
                .path()
                .join("Games")
                .join(platform.dir_name())
                .join(format!("{stem}.{}", platform.extensions()[0]));
            std::fs::write(rom, vec![0u8; 0x150]).expect("write rom");
        }
    }
    d
}

fn config(root: &Path, file: &str, text: &str) {
    let path = root.join("Config").join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn read(root: &Path, file: &str) -> String {
    std::fs::read_to_string(root.join("Config").join(file)).unwrap_or_default()
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn play_and_eject(a: &mut App) {
    a.apply(Action::Insert);
    a.on_core_ready();
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "the cart never started"
    );
    a.apply(Action::Eject);
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "the cart never came back out"
    );
}

fn list(name: &str) -> ShelfKind {
    ShelfKind::List(name.to_string())
}

#[test]
fn recently_played_favourites_and_lists_come_before_the_platform_shelves() {
    let d = card(&["Emerald", "Fusion"], &["Tetris"], &[]);
    config(d.path(), "recent.txt", "GBA/Fusion\n");
    config(d.path(), "favorites.txt", "Tetris\n");
    config(d.path(), "Lists/Zelda.txt", "Emerald\n");
    let a = common::boot(d.path());
    assert_eq!(
        a.shelf_kinds(),
        vec![
            ShelfKind::Recent,
            ShelfKind::Favorites,
            list("Zelda"),
            ShelfKind::Platform(Platform::Gba),
            ShelfKind::Platform(Platform::Gb),
        ]
    );
}

#[test]
fn the_carousel_opens_on_recently_played_when_there_is_one() {
    let d = card(&["Emerald", "Fusion"], &[], &[]);
    config(d.path(), "recent.txt", "GBA/Fusion\n");
    let a = common::boot(d.path());
    assert_eq!(a.shelf_kind(), &ShelfKind::Recent);
    assert_eq!(a.selected_stem(), Some("Fusion"));
    assert_eq!(a.shelf_platform_name(), Some("Recently Played"));

    let fresh = card(&["Emerald", "Fusion"], &[], &[]);
    config(fresh.path(), "Lists/Zelda.txt", "Emerald\n");
    let a = common::boot(fresh.path());
    assert_eq!(
        a.shelf_kind(),
        &ShelfKind::Platform(Platform::Gba),
        "a card nobody has played opened on a list"
    );
}

#[test]
fn a_collection_with_no_carts_on_the_card_is_skipped_like_an_empty_platform() {
    let d = card(&["Emerald"], &["Tetris"], &[]);
    config(d.path(), "favorites.txt", "GBA/Gone\n");
    config(d.path(), "Lists/Empty.txt", "");
    let mut a = common::boot(d.path());
    assert_eq!(
        a.shelf_kinds(),
        vec![
            ShelfKind::Platform(Platform::Gba),
            ShelfKind::Platform(Platform::Gb),
        ]
    );
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_kind(), &ShelfKind::Platform(Platform::Gb));
}

#[test]
fn each_file_in_config_lists_is_a_shelf_named_after_it() {
    let d = card(&["Emerald", "Fusion", "Ruby"], &[], &[]);
    config(
        d.path(),
        "Lists/Zelda.txt",
        "# best first\nFusion\nEmerald\n",
    );
    config(d.path(), "Lists/Couch.txt", "Ruby\n");
    let mut a = common::boot(d.path());
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_kind(), &list("Zelda"));
    assert_eq!(a.shelf_platform_name(), Some("Zelda"));
    assert_eq!(
        a.shelf_stems(),
        ["Fusion", "Emerald"],
        "the list lost its order"
    );
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_kind(), &list("Couch"));
}

#[test]
fn y_favourites_the_cart_under_the_slot_and_says_so_in_the_mouth() {
    let d = card(&["Emerald", "Fusion"], &[], &[]);
    let mut a = common::boot(d.path());
    press(&mut a, Btn::Right);
    press(&mut a, Btn::Y);
    assert_eq!(a.slot_text().as_deref(), Some("Favorited"));
    a.flush_lists();
    assert_eq!(read(d.path(), "favorites.txt"), "GBA/Fusion\n");
    assert!(a.shelf_kinds().contains(&ShelfKind::Favorites));
    assert_eq!(
        a.selected_stem(),
        Some("Fusion"),
        "favouriting moved the shelf"
    );

    press(&mut a, Btn::Y);
    assert_eq!(a.slot_text().as_deref(), Some("Unfavorited"));
    a.flush_lists();
    assert_eq!(read(d.path(), "favorites.txt"), "");
    assert!(!a.shelf_kinds().contains(&ShelfKind::Favorites));

    press(&mut a, Btn::Down);
    assert_ne!(
        a.slot_text().as_deref(),
        Some("Unfavorited"),
        "the note outstayed a jump"
    );
}

#[test]
fn y_on_the_favourites_shelf_removes_the_cart_and_its_neighbour_slides_in() {
    let d = card(&["Emerald", "Fusion", "Ruby"], &[], &[]);
    config(
        d.path(),
        "favorites.txt",
        "GBA/Ruby\nGBA/Emerald\nGBA/Fusion\n",
    );
    let mut a = common::boot(d.path());
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_kind(), &ShelfKind::Favorites);
    assert_eq!(
        a.shelf_stems(),
        ["Emerald", "Fusion", "Ruby"],
        "favourites are not A to Z"
    );
    press(&mut a, Btn::Y);
    assert_eq!(a.shelf_stems(), ["Fusion", "Ruby"]);
    assert_eq!(a.selected_stem(), Some("Fusion"));
    a.flush_lists();
    assert_eq!(read(d.path(), "favorites.txt"), "GBA/Ruby\nGBA/Fusion\n");
}

#[test]
fn unfavouriting_the_last_favourite_moves_to_the_next_shelf() {
    let d = card(&["Emerald", "Fusion"], &[], &[]);
    config(d.path(), "favorites.txt", "GBA/Fusion\n");
    let mut a = common::boot(d.path());
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_kind(), &ShelfKind::Favorites);
    press(&mut a, Btn::Y);
    assert_eq!(a.shelf_kind(), &ShelfKind::Platform(Platform::Gba));
    assert!(a.selected_stem().is_some());
}

#[test]
fn playing_a_game_puts_it_first_in_recently_played() {
    let d = card(&["Emerald", "Fusion"], &[], &[]);
    config(d.path(), "recent.txt", "GBA/Emerald\n");
    let mut a = common::boot(d.path());
    press(&mut a, Btn::R1);
    assert_eq!(a.shelf_kind(), &ShelfKind::Platform(Platform::Gba));
    press(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Fusion"));
    play_and_eject(&mut a);
    a.flush_lists();
    assert_eq!(read(d.path(), "recent.txt"), "GBA/Fusion\nGBA/Emerald\n");
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_kind(), &ShelfKind::Recent);
    assert_eq!(a.shelf_stems(), ["Fusion", "Emerald"]);
    assert_eq!(a.selected_stem(), Some("Fusion"));
}

#[test]
fn recently_played_does_not_reorder_under_you_until_you_leave_it() {
    let d = card(&["Emerald", "Fusion"], &[], &[]);
    config(d.path(), "recent.txt", "GBA/Emerald\nGBA/Fusion\n");
    let mut a = common::boot(d.path());
    assert_eq!(a.shelf_kind(), &ShelfKind::Recent);
    press(&mut a, Btn::Right);
    play_and_eject(&mut a);
    assert_eq!(a.shelf_kind(), &ShelfKind::Recent);
    assert_eq!(
        a.shelf_stems(),
        ["Emerald", "Fusion"],
        "the shelf reshuffled while you were on it"
    );
    assert_eq!(a.selected_stem(), Some("Fusion"));
    press(&mut a, Btn::R1);
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_kind(), &ShelfKind::Recent);
    assert_eq!(a.shelf_stems(), ["Fusion", "Emerald"]);
    assert_eq!(a.selected_stem(), Some("Fusion"));
}

#[test]
fn a_seated_cart_resumes_on_its_platform_shelf_not_recently_played() {
    let d = card(&["Emerald", "Fusion"], &[], &[]);
    config(d.path(), "recent.txt", "GBA/Fusion\nGBA/Emerald\n");
    let a = common::app_playing_in(d.path(), "Emerald");
    assert!(matches!(a.phase(), Phase::Playing { .. }));
    assert_eq!(a.shelf_kind(), &ShelfKind::Platform(Platform::Gba));
    assert_eq!(a.seated_cart().map(|c| c.stem.as_str()), Some("Emerald"));
}

#[test]
fn a_cart_on_two_shelves_is_counted_once() {
    let d = card(&["Emerald"], &[], &[]);
    config(d.path(), "recent.txt", "GBA/Emerald\n");
    config(d.path(), "favorites.txt", "GBA/Emerald\n");
    let a = common::boot(d.path());
    assert!(a.single_cart());
    assert_eq!(a.carts().count(), 1);
}

#[test]
fn a_game_boy_and_a_game_boy_color_cart_with_one_name_each_open_their_own_rom() {
    let d = card(&["Emerald"], &["Tetris"], &["Tetris"]);
    config(d.path(), "favorites.txt", "GBC/Tetris\nGB/Tetris\n");
    let mut a = common::boot(d.path());
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_kind(), &ShelfKind::Favorites);
    press(&mut a, Btn::Right);
    a.apply(Action::Insert);
    assert_eq!(a.seated_cart().map(|c| c.platform), Some(Platform::Gbc));
}

#[test]
fn a_card_with_no_collections_boots_as_it_always_did() {
    let d = card(&["Emerald", "Fusion"], &["Tetris"], &[]);
    let mut state = SlotState {
        clock_set: true,
        ..Default::default()
    };
    state.cart = None;
    write_slot_state(d.path(), &state).unwrap();
    let mut a = App::boot(d.path());
    assert_eq!(a.shelf_kind(), &ShelfKind::Platform(Platform::Gba));
    press(&mut a, Btn::R1);
    assert_eq!(a.shelf_kind(), &ShelfKind::Platform(Platform::Gb));
    press(&mut a, Btn::R1);
    assert_eq!(a.shelf_kind(), &ShelfKind::Platform(Platform::Gba));
    assert!(
        d.path().join("Config/Lists").is_dir(),
        "no Lists folder to find"
    );
}

#[test]
fn a_face_dropped_by_a_restock_goes_back_to_the_pool() {
    let d = card(&["Emerald", "Fusion", "Ruby"], &[], &[]);
    config(d.path(), "favorites.txt", "GBA/Emerald\nGBA/Fusion\n");
    let mut a = common::boot(d.path());
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_kind(), &ShelfKind::Favorites);
    let favourites = a
        .face_wants()
        .first()
        .map(|w| w.shelf)
        .expect("the shelf asked for no faces");
    a.set_cart_face(favourites, 0, slot_ui::TexId::from_raw(7));
    a.set_cart_face(favourites, 1, slot_ui::TexId::from_raw(8));
    press(&mut a, Btn::Y);
    assert!(a.shed_faces().contains(&slot_ui::TexId::from_raw(7)));
    assert_eq!(
        a.face_cart(favourites, 0).map(|c| c.stem.as_str()),
        Some("Fusion")
    );
    assert!(
        a.face_wants().iter().all(|w| w.shelf != favourites),
        "Fusion lost its face when Emerald left"
    );
}
