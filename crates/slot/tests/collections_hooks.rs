const HOOKS: &[(&str, &str, &str)] = &[
    ("app.rs", include_str!("../src/app.rs"), "pub mod shelves;"),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        "shelves::stock(&mut app, root);",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        "if shelves::input(self, action) {",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        "shelves::draw(self, out);",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        "List(&'static str),",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        "collections: shelves::Collections,",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        "if let Some(title) = shelves::search_title(self) {",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        ".get(shelf.index)\n            .filter(|c| c.stem == *stem)",
    ),
    (
        "frontend.rs",
        include_str!("../src/frontend.rs"),
        "crate::app::shelves::sync_faces(self.session.app_mut(), compositor);",
    ),
    (
        "cart_faces.rs",
        include_str!("../src/cart_faces.rs"),
        "if asked_for.is_some_and(|rom| app.face_cart(key.0, key.1).map(|c| &c.rom) != Some(&rom)) {",
    ),
    (
        "slot-ui lib.rs",
        include_str!("../../slot-ui/src/lib.rs"),
        "mod keyboard;",
    ),
    (
        "slot-store lib.rs",
        include_str!("../../slot-store/src/lib.rs"),
        "pub mod collections;",
    ),
    (
        "slot-store lib.rs",
        include_str!("../../slot-store/src/lib.rs"),
        "pub mod find;",
    ),
];

#[test]
fn every_collections_hook_survived_the_merge() {
    for (file, text, hook) in HOOKS {
        assert!(text.contains(hook), "{file}: {hook}");
    }
}
