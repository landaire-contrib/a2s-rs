#![no_main]

use a2s::rules::Rule;
use a2s::rules::arma3::Arma3Rules;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: Vec<Rule>| {
    let _ = Arma3Rules::from_rules(&input);
});
