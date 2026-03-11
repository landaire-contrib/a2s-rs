use std::fs;

use a2s::DeOptions;
use a2s::info::Info;
use a2s::players::Player;
use a2s::rules::Rule;

fn fixture(path: &str) -> Vec<u8> {
    let full = format!("tests/fixtures/{path}");
    fs::read(&full).unwrap_or_else(|e| panic!("failed to read fixture {full}: {e}"))
}

// -- App 320 (Half-Life 2: Deathmatch) --

#[test]
fn info_app320() {
    let data = fixture("320/74_91_118_209_27015_info.bin");
    let info = Info::from_reader(data.as_slice()).unwrap();
    insta::assert_debug_snapshot!(info);
}

#[test]
fn players_app320() {
    let data = fixture("320/74_91_118_209_27015_players.bin");
    let players = Player::from_reader(data.as_slice(), &DeOptions::default()).unwrap();
    insta::assert_debug_snapshot!(players);
}

#[test]
fn rules_app320() {
    let data = fixture("320/74_91_118_209_27015_rules.bin");
    let rules = Rule::from_reader(data.as_slice()).unwrap();
    insta::assert_debug_snapshot!(rules);
}

// -- App 70 (Half-Life) --

#[test]
fn info_app70() {
    let data = fixture("70/coralie_megabrutal_com_27015_info.bin");
    let info = Info::from_reader(data.as_slice()).unwrap();
    insta::assert_debug_snapshot!(info);
}

#[test]
fn players_app70() {
    let data = fixture("70/coralie_megabrutal_com_27015_players.bin");
    let players = Player::from_reader(data.as_slice(), &DeOptions::default()).unwrap();
    insta::assert_debug_snapshot!(players);
}

#[test]
fn rules_app70() {
    let data = fixture("70/coralie_megabrutal_com_27015_rules.bin");
    let rules = Rule::from_reader(data.as_slice()).unwrap();
    insta::assert_debug_snapshot!(rules);
}

// -- Roundtrip: write then parse back --

#[test]
fn info_roundtrip_app320() {
    let data = fixture("320/74_91_118_209_27015_info.bin");
    let info = Info::from_reader(data.as_slice()).unwrap();
    let bytes = info.to_bytes();
    // to_bytes includes the 0xFFFFFFFF + 0x49 header, from_reader expects starting at 0x49
    let reparsed = Info::from_reader(&bytes[4..]).unwrap();
    insta::assert_debug_snapshot!(reparsed);
}
