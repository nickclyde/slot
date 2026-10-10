use std::path::Path;
use std::process::Command;

#[test]
fn the_wifi_script_passes_its_card_tests() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../card/test-slotnet.sh");
    let out = Command::new("sh").arg(&script).output().expect("run sh");
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
