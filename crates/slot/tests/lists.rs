mod common;

use std::path::Path;

use slot::app::App;
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

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn power_off(a: &mut App) {
    a.apply(Action::PowerHold);
    assert!(a.powering_off());
}

const GBA: &str = "Game Boy Advance";
const GB: &str = "Game Boy";

#[test]
fn lists_come_after_favorites_and_before_the_platform_shelves() {
    let d = card(&["Emerald", "Fusion"], &["Tetris"], &[]);
    config(d.path(), "favorites.txt", "GB/Tetris\n");
    config(d.path(), "Lists/Zelda.txt", "Emerald\n");
    let a = common::boot(d.path());
    assert_eq!(a.shelf_names(), ["Favorites", "Zelda", GBA, GB]);
}

#[test]
fn a_card_nobody_has_played_still_opens_on_its_platform_shelf() {
    let d = card(&["Emerald", "Fusion"], &[], &[]);
    config(d.path(), "Lists/Zelda.txt", "Emerald\n");
    let a = common::boot(d.path());
    assert_eq!(a.shelf_platform_name(), Some(GBA));
}

#[test]
fn a_list_with_no_carts_on_the_card_is_skipped_like_an_empty_platform() {
    let d = card(&["Emerald"], &["Tetris"], &[]);
    config(d.path(), "Lists/Gone.txt", "GBA/Gone\n");
    config(d.path(), "Lists/Empty.txt", "");
    let mut a = common::boot(d.path());
    assert_eq!(a.shelf_names(), [GBA, GB]);
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_platform_name(), Some(GB));
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
    assert_eq!(a.shelf_platform_name(), Some("Zelda"));
    assert_eq!(
        a.shelf_stems(),
        ["Fusion", "Emerald"],
        "the list lost its order"
    );
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_platform_name(), Some("Couch"));
}

#[test]
fn powering_off_on_a_list_boots_back_onto_it() {
    let d = card(&["Emerald", "Fusion", "Ruby"], &[], &[]);
    config(d.path(), "Lists/Zelda.txt", "Fusion\nEmerald\n");
    let mut a = common::boot(d.path());
    press(&mut a, Btn::L1);
    press(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Emerald"));
    power_off(&mut a);

    let a = App::boot(d.path());
    assert_eq!(a.shelf_platform_name(), Some("Zelda"));
    assert_eq!(a.selected_stem(), Some("Emerald"));
}

#[test]
fn y_on_a_list_favourites_the_cart_without_leaving_the_list() {
    let d = card(&["Emerald", "Fusion"], &[], &[]);
    config(d.path(), "Lists/Zelda.txt", "Fusion\nEmerald\n");
    let mut a = common::boot(d.path());
    press(&mut a, Btn::L1);
    press(&mut a, Btn::Y);
    assert_eq!(a.shelf_platform_name(), Some("Zelda"));
    assert_eq!(a.selected_stem(), Some("Fusion"));
    assert_eq!(a.shelf_names(), ["Favorites", "Zelda", GBA]);
}

#[test]
fn a_cart_on_a_list_and_its_platform_is_counted_once() {
    let d = card(&["Emerald"], &[], &[]);
    config(d.path(), "Lists/Zelda.txt", "GBA/Emerald\n");
    let a = common::boot(d.path());
    assert!(a.single_cart());
    assert_eq!(a.carts().count(), 1);
}

#[test]
fn a_game_boy_and_a_game_boy_color_cart_with_one_name_each_open_their_own_rom() {
    let d = card(&["Emerald"], &["Tetris"], &["Tetris"]);
    config(d.path(), "Lists/Blocks.txt", "GBC/Tetris\nGB/Tetris\n");
    let mut a = common::boot(d.path());
    press(&mut a, Btn::L1);
    assert_eq!(a.shelf_platform_name(), Some("Blocks"));
    press(&mut a, Btn::Right);
    a.apply(Action::Insert);
    assert_eq!(a.seated_cart().map(|c| c.platform), Some(Platform::Gb));
}

#[test]
fn a_card_with_no_lists_boots_as_it_always_did() {
    let d = card(&["Emerald", "Fusion"], &["Tetris"], &[]);
    let state = SlotState {
        clock_set: true,
        ..Default::default()
    };
    write_slot_state(d.path(), &state).unwrap();
    let mut a = App::boot(d.path());
    assert_eq!(a.shelf_platform_name(), Some(GBA));
    press(&mut a, Btn::R1);
    assert_eq!(a.shelf_platform_name(), Some(GB));
    press(&mut a, Btn::R1);
    assert_eq!(a.shelf_platform_name(), Some(GBA));
    assert!(
        d.path().join("Config/Lists").is_dir(),
        "no Lists folder to find"
    );
}
