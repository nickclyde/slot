#![cfg(feature = "cheevos")]

use slot::cheevos::net::{configured, why, WIFI_FILE};

fn root_with(text: Option<&str>) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    if let Some(text) = text {
        std::fs::create_dir_all(d.path().join("Config")).unwrap();
        std::fs::write(d.path().join(WIFI_FILE), text).unwrap();
    }
    d
}

#[test]
fn wifi_only_counts_as_configured_once_it_names_a_network() {
    assert!(!configured(root_with(None).path()));
    assert!(!configured(root_with(Some("# ssid = Home\n")).path()));
    assert!(!configured(root_with(Some("ssid =\npsk = x\n")).path()));
    assert!(configured(root_with(Some("ssid = Home\n")).path()));
}

#[test]
fn script_exit_codes_explain_themselves() {
    assert_eq!(
        why(Some(5)),
        "no DHCP client, set address in Config/wifi.txt"
    );
    assert_eq!(why(Some(4)), "link play has the radio");
    assert_eq!(why(None), "the script failed");
}
