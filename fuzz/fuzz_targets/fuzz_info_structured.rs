#![no_main]

use a2s::info::Info;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: Info| {
    let bytes = input.to_bytes();
    // to_bytes includes the 0xFFFFFFFF header; from_reader expects starting at 0x49
    let _ = Info::from_reader(&bytes[4..]);
});
