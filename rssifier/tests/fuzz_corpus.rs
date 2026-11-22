//! The stored corpus of the fuzz target in `fuzz/`, played with stable Rust:
//! every input the fuzzer kept, and every one that once broke a property,
//! checks the same properties here as there.

use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

#[path = "../fuzz/checks.rs"]
mod checks;

/// How long one input may take. The fuzzer gives up after ten seconds, in a
/// build with optimizations; a test is built without.
const MAX_TIME: Duration = Duration::from_secs(60);

#[test]
fn reading_pages_keeps_to_its_properties() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/corpus/page");
    let mut inputs: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    inputs.sort();
    assert!(!inputs.is_empty(), "no corpus in {}", dir.display());
    for input in &inputs {
        let data = fs::read(input).unwrap();
        let started = Instant::now();
        checks::page(&data);
        let took = started.elapsed();
        assert!(took < MAX_TIME, "{} took {took:?}", input.display());
    }
    println!("page: {} inputs", inputs.len());
}
