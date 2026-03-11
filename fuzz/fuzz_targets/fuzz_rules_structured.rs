#![no_main]

use a2s::rules::Rule;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: Vec<Rule>| {
    let bytes = Rule::vec_to_bytes(input);
    // vec_to_bytes includes the 0xFFFFFFFF header; from_reader expects starting at 0x45
    let _ = Rule::from_reader(&bytes[4..]);
});
