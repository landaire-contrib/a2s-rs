use std::fs;

use a2s::DeOptions;
use a2s::info::Info;
use a2s::players::Player;
use a2s::rules::Rule;
#[cfg(feature = "arma3")]
use a2s::rules::arma3::Arma3Rules;

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

#[cfg(feature = "arma3")]
fn test_arma3_rules(path: &str) {
    let data = fixture(path);
    let rules = Rule::from_reader(data.as_slice()).unwrap();
    let arma3 = Arma3Rules::from_rules(&rules).unwrap();
    let name = format!("{}_arma3", snap_name(path));
    insta::assert_debug_snapshot!(name, arma3);
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

// -- DayOne (Chernarus) --

#[test]
fn info_dayone_chernarus() {
    test_info("0/172_111_51_218_2402_info.bin");
}

#[test]
fn players_dayone_chernarus() {
    test_players("0/172_111_51_218_2402_players.bin", &DeOptions::default());
}

#[test]
fn rules_dayone_chernarus() {
    test_rules("0/172_111_51_218_2402_rules.bin");
}

#[test]
fn info_roundtrip_dayone_chernarus() {
    test_info_roundtrip("0/172_111_51_218_2402_info.bin");
}

// -- DayOne (Livonia) --

#[test]
fn info_dayone_livonia() {
    test_info("0/172_111_51_218_2302_info.bin");
}

#[test]
fn players_dayone_livonia() {
    test_players("0/172_111_51_218_2302_players.bin", &DeOptions::default());
}

#[test]
fn rules_dayone_livonia() {
    test_rules("0/172_111_51_218_2302_rules.bin");
}

#[test]
fn info_roundtrip_dayone_livonia() {
    test_info_roundtrip("0/172_111_51_218_2302_info.bin");
}

// -- DayOne (Namalsk #1) --

#[test]
fn info_dayone_namalsk1() {
    test_info("0/172_111_51_213_2302_info.bin");
}

#[test]
fn players_dayone_namalsk1() {
    test_players("0/172_111_51_213_2302_players.bin", &DeOptions::default());
}

#[test]
fn rules_dayone_namalsk1() {
    test_rules("0/172_111_51_213_2302_rules.bin");
}

#[test]
fn info_roundtrip_dayone_namalsk1() {
    test_info_roundtrip("0/172_111_51_213_2302_info.bin");
}

// -- DayOne (Namalsk #2) --

#[test]
fn info_dayone_namalsk2() {
    test_info("0/172_111_51_213_2402_info.bin");
}

#[test]
fn players_dayone_namalsk2() {
    test_players("0/172_111_51_213_2402_players.bin", &DeOptions::default());
}

#[test]
fn rules_dayone_namalsk2() {
    test_rules("0/172_111_51_213_2402_rules.bin");
}

#[test]
fn info_roundtrip_dayone_namalsk2() {
    test_info_roundtrip("0/172_111_51_213_2402_info.bin");
}

// -- Arma 3 rules parsing --

#[test]
#[cfg(feature = "arma3")]
fn arma3_rules_dayone_chernarus() {
    test_arma3_rules("0/172_111_51_218_2402_rules.bin");
}

#[test]
#[cfg(feature = "arma3")]
fn arma3_rules_dayone_livonia() {
    test_arma3_rules("0/172_111_51_218_2302_rules.bin");
}

#[test]
#[cfg(feature = "arma3")]
fn arma3_rules_dayone_namalsk1() {
    test_arma3_rules("0/172_111_51_213_2302_rules.bin");
}

#[test]
#[cfg(feature = "arma3")]
fn arma3_rules_dayone_namalsk2() {
    test_arma3_rules("0/172_111_51_213_2402_rules.bin");
}

#[test]
#[cfg(feature = "arma3")]
fn arma3_rules_dayzero_deerisle() {
    test_arma3_rules("0/51_38_89_140_2302_rules.bin");
}

#[test]
#[cfg(feature = "arma3")]
fn arma3_rules_karmakrew_chernarus() {
    test_arma3_rules("0/193_25_252_55_27016_rules.bin");
}

// -- Malformed server responses --

/// Server sends ASCII '1' (0x31) for the VAC boolean instead of binary 1 (0x01).
/// This should produce an InvalidBool error rather than silently misinterpreting
/// the version string.
#[test]
fn info_rejects_invalid_vac_byte() {
    let data = fixture("unknown/115_190_141_50_27021_info.bin");
    let err = Info::from_reader(data.as_slice()).unwrap_err();
    assert!(
        err.to_string().contains("vac"),
        "expected InvalidBool error for vac field, got: {err}"
    );
}
