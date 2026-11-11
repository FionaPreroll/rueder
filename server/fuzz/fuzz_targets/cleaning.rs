#![no_main]

#[path = "../checks.rs"]
#[allow(dead_code)]
mod checks;

libfuzzer_sys::fuzz_target!(|data: &[u8]| checks::cleaning(data));
