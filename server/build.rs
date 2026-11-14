//! What the program says about how it was built: the commit it came from and
//! when that was made, how it was built, and for which machine. `rueder --version` and the
//! status page show it, so an admin can tell which rueder is running.
//!
//! Where there is a git directory, the facts come from it. Where there is none,
//! the image for one, whoever builds passes them in the environment
//! (`RUEDER_COMMIT` and its kin); what is missing stays empty and the status
//! says so.

use std::{env, process::Command};

/// What is passed in where git cannot be asked.
const FROM_ENV: [&str; 6] = [
    "RUEDER_COMMIT",
    "RUEDER_COMMITTED_AT",
    "RUEDER_BRANCH",
    "RUEDER_TAG",
    "RUEDER_DIRTY",
    "RUEDER_CI_RUN",
];

fn main() {
    // The git directory, so a new commit means a new build info. Nothing else
    // here depends on the sources, which cargo watches anyway.
    for path in [".git/HEAD", ".git/refs", "../.git/HEAD", "../.git/refs"] {
        println!("cargo:rerun-if-changed={path}");
    }
    for name in FROM_ENV {
        println!("cargo:rerun-if-env-changed={name}");
    }

    let commit = given("RUEDER_COMMIT").or_else(|| git(&["rev-parse", "HEAD"]));
    // In UTC, as `2026-11-13T09:12:00Z`: the same commit is the same build.
    let committed_at = given("RUEDER_COMMITTED_AT").or_else(|| {
        let format = "--date=format-local:%Y-%m-%dT%H:%M:%SZ";
        git(&["show", "--no-patch", format, "--format=%cd", "HEAD"])
    });
    let branch = given("RUEDER_BRANCH").or_else(|| git(&["rev-parse", "--abbrev-ref", "HEAD"]));
    let tag = given("RUEDER_TAG").or_else(|| git(&["describe", "--tags", "--exact-match"]));
    let dirty = given("RUEDER_DIRTY").unwrap_or_else(|| {
        // Changes that are not committed: the build is not the commit itself.
        match git(&["status", "--porcelain"]) {
            Some(changed) => (!changed.is_empty()).to_string(),
            None => String::new(),
        }
    });

    let (commit, committed_at, branch, tag) = (
        commit.unwrap_or_default(),
        committed_at.unwrap_or_default(),
        branch.unwrap_or_default(),
        tag.unwrap_or_default(),
    );
    set(
        "RUEDER_VERSION_LINE",
        line(&commit, &branch, &tag, dirty == "true", &committed_at),
    );
    set("RUEDER_COMMIT", commit);
    set("RUEDER_COMMITTED_AT", committed_at);
    set("RUEDER_BRANCH", branch);
    set("RUEDER_TAG", tag);
    set("RUEDER_DIRTY", dirty);
    set("RUEDER_CI_RUN", given("RUEDER_CI_RUN").unwrap_or_default());
    for name in ["TARGET", "PROFILE", "OPT_LEVEL"] {
        set(
            &format!("RUEDER_{name}"),
            env::var(name).unwrap_or_default(),
        );
    }
}

/// What `rueder --version` says: the version of the crate, and what tells this
/// build from another of the same version. Written here, where the parts are
/// known, so that the program carries it as one line.
fn line(commit: &str, branch: &str, tag: &str, dirty: bool, committed_at: &str) -> String {
    let mut line = env::var("CARGO_PKG_VERSION").unwrap_or_default();
    if !commit.is_empty() {
        let short: String = commit.chars().take(12).collect();
        line.push_str(&format!(" ({short}"));
        if dirty {
            line.push_str(", dirty");
        }
        match (tag.is_empty(), branch.is_empty()) {
            (false, _) => line.push_str(&format!(", {tag}")),
            (true, false) => line.push_str(&format!(", {branch}")),
            _ => {}
        }
        line.push(')');
    }
    if !committed_at.is_empty() {
        line.push_str(&format!(", committed {committed_at}"));
    }
    if let Ok(target) = env::var("TARGET") {
        line.push_str(&format!(" for {target}"));
    }
    line
}

fn set(name: &str, value: String) {
    println!("cargo:rustc-env={name}={value}");
}

/// What the environment says, if it says anything.
fn given(name: &str) -> Option<String> {
    let value = env::var(name).ok()?;
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

/// What git says, or nothing: no git, no repository, or a commit nobody tagged.
fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .env("TZ", "UTC")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    (!text.is_empty()).then_some(text)
}
