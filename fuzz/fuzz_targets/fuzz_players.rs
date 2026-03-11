#![no_main]

use a2s::DeOptions;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = a2s::players::Player::from_reader(data, &DeOptions::default());
    let _ = a2s::players::Player::from_reader(data, &DeOptions { the_ship: true });
});
