use std::fs;

use a2s::DeOptions;
use a2s::info::Info;
use a2s::players::Player;
use a2s::rules::Rule;

fn fixture(path: &str) -> Vec<u8> {
    let full = format!("tests/fixtures/{path}");
    fs::read(&full).unwrap_or_else(|e| panic!("failed to read fixture {full}: {e}"))
}

/// Derive a snapshot name from a fixture path:
/// `320/74_91_118_209_27015_info.bin` -> `320__74_91_118_209_27015_info`
fn snap_name(fixture_path: &str) -> String {
    fixture_path
        .strip_suffix(".bin")
        .unwrap_or(fixture_path)
        .replace('/', "__")
}

fn test_info(path: &str) {
    let data = fixture(path);
    let info = Info::from_reader(data.as_slice()).unwrap();
    insta::assert_debug_snapshot!(snap_name(path), info);
}

fn test_info_roundtrip(path: &str) {
    let data = fixture(path);
    let info = Info::from_reader(data.as_slice()).unwrap();
    let bytes = info.to_bytes();
    // to_bytes includes the 0xFFFFFFFF + 0x49 header, from_reader expects starting at 0x49
    let reparsed = Info::from_reader(&bytes[4..]).unwrap();
    let name = format!("{}_roundtrip", snap_name(path));
    insta::assert_debug_snapshot!(name, reparsed);
}

fn test_players(path: &str, opts: &DeOptions) {
    let data = fixture(path);
    let players = Player::from_reader(data.as_slice(), opts).unwrap();
    insta::assert_debug_snapshot!(snap_name(path), players);
}

fn test_rules(path: &str) {
    let data = fixture(path);
    let rules = Rule::from_reader(data.as_slice()).unwrap();
    insta::assert_debug_snapshot!(snap_name(path), rules);
}

// -- App 320 (Half-Life 2: Deathmatch) --

#[test]
fn info_app320() {
    test_info("320/74_91_118_209_27015_info.bin");
}

#[test]
fn players_app320() {
    test_players("320/74_91_118_209_27015_players.bin", &DeOptions::default());
}

#[test]
fn rules_app320() {
    test_rules("320/74_91_118_209_27015_rules.bin");
}

#[test]
fn info_roundtrip_app320() {
    test_info_roundtrip("320/74_91_118_209_27015_info.bin");
}

// -- App 70 (Half-Life) --

#[test]
fn info_app70() {
    test_info("70/coralie_megabrutal_com_27015_info.bin");
}

#[test]
fn players_app70() {
    test_players(
        "70/coralie_megabrutal_com_27015_players.bin",
        &DeOptions::default(),
    );
}

#[test]
fn rules_app70() {
    test_rules("70/coralie_megabrutal_com_27015_rules.bin");
}

// -- DayZ (KarmaKrew) --

#[test]
fn info_dayz_karmakrew_alteria() {
    test_info("0/193_25_252_72_27016_info.bin");
}

#[test]
fn players_dayz_karmakrew_alteria() {
    test_players("0/193_25_252_72_27016_players.bin", &DeOptions::default());
}

#[test]
fn info_dayz_karmakrew_chernarus() {
    test_info("0/193_25_252_55_27016_info.bin");
}

#[test]
fn rules_dayz_karmakrew_chernarus() {
    test_rules("0/193_25_252_55_27016_rules.bin");
}

#[test]
fn info_roundtrip_dayz_karmakrew_alteria() {
    test_info_roundtrip("0/193_25_252_72_27016_info.bin");
}

// -- DayZ (DayZero - Deer Isle) --

#[test]
fn info_dayz_dayzero_deerisle() {
    test_info("0/51_38_89_140_2302_info.bin");
}

#[test]
fn players_dayz_dayzero_deerisle() {
    test_players("0/51_38_89_140_2302_players.bin", &DeOptions::default());
}

#[test]
fn rules_dayz_dayzero_deerisle() {
    test_rules("0/51_38_89_140_2302_rules.bin");
}

#[test]
fn info_roundtrip_dayz_dayzero_deerisle() {
    test_info_roundtrip("0/51_38_89_140_2302_info.bin");
}
