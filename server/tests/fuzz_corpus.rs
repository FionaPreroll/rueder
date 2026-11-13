//! The stored corpus of the fuzz targets in `fuzz/`, played with stable Rust:
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

fn play(target: &str, check: fn(&[u8])) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz/corpus")
        .join(target);
    let mut inputs: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    inputs.sort();
    assert!(!inputs.is_empty(), "no corpus in {}", dir.display());
    for input in &inputs {
        let data = fs::read(input).unwrap();
        let started = Instant::now();
        check(&data);
        let took = started.elapsed();
        assert!(took < MAX_TIME, "{} took {took:?}", input.display());
    }
    println!("{target}: {} inputs", inputs.len());
}

#[test]
fn the_cleaning_keeps_to_its_properties() {
    play("cleaning", checks::cleaning);
}

#[test]
fn feeds_keep_to_their_properties() {
    play("feed", checks::feed);
}

#[test]
fn the_image_proxy_keeps_to_its_properties() {
    play("image", checks::image);
}

#[test]
fn requests_of_apps_keep_to_their_properties() {
    play("greader", checks::greader);
}
