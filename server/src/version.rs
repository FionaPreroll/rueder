//! What this build is: which commit it came from, when and how it was built,
//! and where it runs. `build.rs` puts the facts into the program when it is
//! built; what is not known is empty, as in a build from a source archive
//! without a git directory.
//!
//! None of it is a secret: a commit, a branch, a tag and a build date say what
//! an admin needs to tell one rueder from another, and the status page shows
//! them to admins alone.

use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

/// The version of the crate, which is what `rueder --version` says.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where a program that runs in a container leaves its mark; podman's is the
/// second. The image sets `RUEDER_CONTAINER` besides, for a runtime that leaves
/// neither.
const MARKS: [&str; 2] = ["/.dockerenv", "/run/.containerenv"];

/// How this rueder was built and where it runs.
///
/// Part of what `GET /api/status` answers, for admins only.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(rename = "BuildInfo")]
pub struct Build {
    /// The version of the crate.
    pub version: &'static str,
    /// The commit it was built from; empty where there was no git directory.
    pub commit: &'static str,
    /// The branch that commit was on.
    pub branch: &'static str,
    /// The tag on that commit, if it has one.
    pub tag: &'static str,
    /// Something was changed and not committed when this was built.
    pub dirty: bool,
    /// When it was built, as `2026-11-13T09:12:00Z`.
    pub built_at: &'static str,
    /// The machine it was built for, as `x86_64-unknown-linux-gnu`.
    pub target: &'static str,
    /// `release` or `debug`, and how hard the compiler tried (`3`, `z`, `0`).
    pub profile: &'static str,
    pub opt_level: &'static str,
    /// The run of the CI that built it, as an address; empty for a build by hand.
    pub ci_run: &'static str,
    /// It runs in a container, as the image of the project does.
    pub container: bool,
}

/// What this build is. The facts are in the program; only where it runs is
/// looked up, which is two paths and an environment variable.
pub fn build() -> Build {
    Build {
        version: VERSION,
        commit: env!("RUEDER_COMMIT"),
        branch: env!("RUEDER_BRANCH"),
        tag: env!("RUEDER_TAG"),
        dirty: matches!(env!("RUEDER_DIRTY"), "true"),
        built_at: env!("RUEDER_BUILT_AT"),
        target: env!("RUEDER_TARGET"),
        profile: env!("RUEDER_PROFILE"),
        opt_level: env!("RUEDER_OPT_LEVEL"),
        ci_run: env!("RUEDER_CI_RUN"),
        container: in_container(),
    }
}

fn in_container() -> bool {
    if std::env::var("RUEDER_CONTAINER").is_ok_and(|value| value == "1") {
        return true;
    }
    MARKS.iter().any(|mark| Path::new(mark).exists())
}

/// What `rueder --version` says: the version, and what tells this build from
/// another of the same version. `build.rs` puts it together.
pub const LINE: &str = env!("RUEDER_VERSION_LINE");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_was_built_is_in_the_program() {
        let build = build();
        assert_eq!(build.version, VERSION);
        // The tests are a debug build, whatever else is unknown here.
        assert_eq!(build.profile, "debug");
        assert!(build.target.contains('-'), "{}", build.target);
        assert!(build.built_at.ends_with('Z'), "{}", build.built_at);
        assert!(LINE.starts_with(VERSION), "{LINE}");
    }
}
