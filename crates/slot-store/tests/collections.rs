use slot_store::collections::{
    hits, parse_entries, played, read_lists, render, toggle, Index, LISTS_DIR, RECENT_MAX,
};
use slot_store::{Cart, Platform};
use std::path::PathBuf;

fn cart(platform: Platform, stem: &str) -> Cart {
    Cart {
        platform,
        stem: stem.to_string(),
        rom: PathBuf::from(format!("Games/{}/{stem}.gba", platform.dir_name())),
        label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}

fn card() -> Vec<Cart> {
    vec![
        cart(Platform::Gba, "Metroid Fusion (USA)"),
        cart(Platform::Gba, "Pokemon - Emerald Version (USA, Europe)"),
        cart(Platform::Gba, "Tetris"),
        cart(Platform::Gb, "Tetris"),
        cart(Platform::Gbc, "Mr. Driller"),
    ]
}

fn key(platform: Platform, stem: &str) -> Option<(Platform, String)> {
    Some((platform, stem.to_string()))
}

#[test]
fn a_list_line_finds_its_cart_by_platform_and_stem_by_bare_stem_or_by_rom_name() {
    let index = Index::new(&card());
    let fusion = key(Platform::Gba, "Metroid Fusion (USA)");
    assert_eq!(index.resolve("GBA/Metroid Fusion (USA)"), fusion);
    assert_eq!(index.resolve("Metroid Fusion (USA)"), fusion);
    assert_eq!(index.resolve("Metroid Fusion (USA).gba"), fusion);
    assert_eq!(index.resolve("Games/GBA/Metroid Fusion (USA).gba"), fusion);
    assert_eq!(
        index.resolve(r"E:\Games\GBA\Metroid Fusion (USA).gba"),
        fusion
    );
    assert_eq!(
        index.resolve("Mr. Driller"),
        key(Platform::Gbc, "Mr. Driller")
    );
}

#[test]
fn list_lines_match_whatever_their_case() {
    let index = Index::new(&card());
    assert_eq!(
        index.resolve("gba/metroid fusion (usa).GBA"),
        key(Platform::Gba, "Metroid Fusion (USA)")
    );
}

#[test]
fn a_line_without_its_region_tag_finds_the_only_cart_it_could_mean() {
    let index = Index::new(&card());
    assert_eq!(
        index.resolve("Metroid Fusion"),
        key(Platform::Gba, "Metroid Fusion (USA)")
    );
    assert_eq!(
        index.resolve("pokemon emerald version"),
        key(Platform::Gba, "Pokemon - Emerald Version (USA, Europe)")
    );
}

#[test]
fn a_bare_stem_on_two_platforms_picks_the_game_boy_advance_one() {
    let index = Index::new(&card());
    assert_eq!(index.resolve("Tetris"), key(Platform::Gba, "Tetris"));
    assert_eq!(index.resolve("GB/Tetris"), key(Platform::Gb, "Tetris"));
    assert_eq!(index.resolve("Tetris.gb"), key(Platform::Gba, "Tetris"));
}

#[test]
fn comments_blank_lines_and_a_byte_order_mark_are_ignored() {
    let index = Index::new(&card());
    assert_eq!(index.resolve("# Tetris"), None);
    assert_eq!(index.resolve("   "), None);
    assert_eq!(
        index.resolve("\u{feff}Tetris"),
        key(Platform::Gba, "Tetris")
    );
    let entries = parse_entries("\u{feff}GB/Tetris\n\n# mine\n", &index);
    assert_eq!(hits(&entries), vec![(Platform::Gb, "Tetris".to_string())]);
}

#[test]
fn a_line_naming_no_cart_on_the_card_is_skipped_but_kept_when_favourites_are_written() {
    let index = Index::new(&card());
    let mut entries = parse_entries("# mine\nZelda\ngba/tetris\n", &index);
    assert_eq!(hits(&entries), vec![(Platform::Gba, "Tetris".to_string())]);
    assert!(toggle(&mut entries, Platform::Gbc, "Mr. Driller"));
    assert_eq!(
        render(&entries),
        "# mine\nZelda\nGBA/Tetris\nGBC/Mr. Driller\n"
    );
}

#[test]
fn favouriting_twice_takes_the_favourite_back() {
    let index = Index::new(&card());
    let mut entries = parse_entries("GBA/Tetris\n", &index);
    assert!(toggle(&mut entries, Platform::Gb, "Tetris"));
    assert!(!toggle(&mut entries, Platform::Gba, "Tetris"));
    assert_eq!(render(&entries), "GB/Tetris\n");
}

#[test]
fn recently_played_puts_the_newest_first_and_forgets_past_the_cap() {
    let carts: Vec<Cart> = (0..RECENT_MAX + 5)
        .map(|i| cart(Platform::Gba, &format!("Game {i}")))
        .collect();
    let mut entries = Vec::new();
    for c in &carts {
        assert!(played(&mut entries, c.platform, &c.stem));
    }
    let order = hits(&entries);
    assert_eq!(order.len(), RECENT_MAX);
    assert_eq!(order[0].1, format!("Game {}", RECENT_MAX + 4));
    assert_eq!(order[RECENT_MAX - 1].1, "Game 5");
}

#[test]
fn playing_a_recent_game_again_moves_it_to_the_front_instead_of_listing_it_twice() {
    let index = Index::new(&card());
    let mut entries = parse_entries("GBA/Tetris\nGB/Tetris\nGBC/Mr. Driller\n", &index);
    assert!(played(&mut entries, Platform::Gbc, "Mr. Driller"));
    assert_eq!(render(&entries), "GBC/Mr. Driller\nGBA/Tetris\nGB/Tetris\n");
    assert!(!played(&mut entries, Platform::Gbc, "Mr. Driller"));
}

#[test]
fn a_list_keeps_its_written_order_and_drops_repeats() {
    let index = Index::new(&card());
    let entries = parse_entries("Tetris\nMr. Driller\nGBA/Tetris\nGB/Tetris\n", &index);
    assert_eq!(
        hits(&entries),
        vec![
            (Platform::Gba, "Tetris".to_string()),
            (Platform::Gbc, "Mr. Driller".to_string()),
            (Platform::Gb, "Tetris".to_string()),
        ]
    );
}

#[test]
fn each_text_file_in_the_lists_folder_is_a_list_named_after_it_in_name_order() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join(LISTS_DIR);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Zelda.txt"), "Tetris\n").unwrap();
    std::fs::write(dir.join("couch co-op.TXT"), "Mr. Driller\n").unwrap();
    std::fs::write(dir.join(".hidden.txt"), "Tetris\n").unwrap();
    std::fs::write(dir.join("notes.md"), "Tetris\n").unwrap();
    let lists = read_lists(root.path(), &Index::new(&card()));
    let names: Vec<&str> = lists.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["couch co-op", "Zelda"]);
    assert_eq!(lists[1].carts, vec![(Platform::Gba, "Tetris".to_string())]);
}
