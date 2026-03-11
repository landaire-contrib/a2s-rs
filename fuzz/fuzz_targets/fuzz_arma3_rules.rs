#![no_main]

use a2s::rules::Rule;
use a2s::rules::arma3::Arma3Rules;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(rules) = Rule::from_reader(data) {
        let _ = Arma3Rules::from_rules(&rules);
    }
});
