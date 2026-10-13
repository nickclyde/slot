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
        "shelves::recorded(self, cart.as_deref());",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        "shelves::draw(self, out);",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        "if let Some(note) = shelves::shown_note(self) {",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        "let mut shed = shelves::take_shed(self);",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        ".filter(|(kind, _)| kind.platform().is_some())",
    ),
    (
        "app.rs",
        include_str!("../src/app.rs"),
        "self.shelf().find(stem).map(|(cart, _)| cart)",
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
        "slot-ui shelf.rs",
        include_str!("../../slot-ui/src/shelf.rs"),
        "mod restock;",
    ),
    (
        "slot-ui shelf.rs",
        include_str!("../../slot-ui/src/shelf.rs"),
        "self.pitch.unwrap_or(",
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
