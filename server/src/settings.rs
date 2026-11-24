//! The settings of `rueder serve` that belong to no module of their own, read
//! from the environment once at the start. A value that is none of its
//! setting stops the start, rather than quietly being the default: a switch
//! is `on` or `off`, and `APP_API=false` is a mistake, not `on`. A setting
//! that is empty counts as not set, as an empty line in `.env` gives.

use std::path::PathBuf;

use anyhow::Context;
use axum::http::HeaderName;

use crate::{http_client::Net, images, status};

/// Where the server listens if `LISTEN_ADDR` does not say.
pub const LISTEN_ADDR: &str = "127.0.0.1:3000";
/// Where the built page is if `STATIC_DIR` does not say.
const STATIC_DIR: &str = "../web/build";
/// Where the image proxy keeps its images if `IMAGE_CACHE_DIR` does not say.
const IMAGE_CACHE_DIR: &str = "image-cache";

/// What the operator turns on or off for the whole server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Switches {
    /// Whether the server takes reports of errors from the browsers of readers
    /// who agreed to send them (`CLIENT_REPORTS`, on as rueder comes).
    pub reports: bool,
    /// Whether the right of an admin counts only in a session that began with
    /// a second factor (`ADMIN_SECOND_FACTOR=required`).
    pub admin_second_factor: bool,
    /// Whether readers may report feeds and articles (`CONTENT_REPORTS`).
    pub content_reports: bool,
    /// Whether apps of other makers may log in with app passwords, through
    /// the Google Reader API (`APP_API`, on as rueder comes).
    pub app_api: bool,
    /// Whether a cache in front of rueder, such as a CDN, may keep images
    /// (`IMAGES_PUBLIC`): they come without a session then, and apps get
    /// addresses that are the same for all of them.
    pub images_public: bool,
}

impl Default for Switches {
    fn default() -> Self {
        Self {
            reports: true,
            admin_second_factor: false,
            content_reports: false,
            app_api: true,
            images_public: false,
        }
    }
}

/// The settings, as the environment gives them.
#[derive(Debug)]
pub struct Settings {
    pub listen_addr: String,
    /// Where the built page is.
    pub static_dir: PathBuf,
    /// Whether the server fetches feeds (`FETCH`, on as rueder comes).
    pub fetch: bool,
    /// Networks the hardened client fetches from although they are internal
    /// (`FETCH_ALLOW_NETS`, such as `192.168.1.0/24, fd00::/8`).
    pub allow: Vec<Net>,
    /// The header a reverse proxy names the sender's address in
    /// (`CLIENT_IP_HEADER`).
    pub client_ip_header: Option<HeaderName>,
    /// From what size of the database the status page warns
    /// (`DATABASE_WARN_GB`).
    pub database_warn_bytes: Option<i64>,
    /// Where the image proxy keeps its images and its key (`IMAGE_CACHE_DIR`,
    /// `IMAGE_CACHE_MB`, `IMAGE_PROXY_KEY`).
    pub images: images::Config,
    pub switches: Switches,
}

impl Settings {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// The settings `var` gives, with the default for what it leaves out.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let set = |name: &str| {
            var(name)
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let switch = |name: &str, default: bool| -> anyhow::Result<bool> {
            match set(name).as_deref() {
                None => Ok(default),
                Some("on") => Ok(true),
                Some("off") => Ok(false),
                Some(other) => anyhow::bail!("{name} is `on` or `off`, not {other:?}"),
            }
        };
        let defaults = Switches::default();
        let switches = Switches {
            reports: switch("CLIENT_REPORTS", defaults.reports)?,
            admin_second_factor: match set("ADMIN_SECOND_FACTOR").as_deref() {
                None | Some("off") => false,
                Some("required") => true,
                Some(other) => {
                    anyhow::bail!("ADMIN_SECOND_FACTOR is `required` or `off`, not {other:?}")
                }
            },
            content_reports: switch("CONTENT_REPORTS", defaults.content_reports)?,
            app_api: switch("APP_API", defaults.app_api)?,
            images_public: switch("IMAGES_PUBLIC", defaults.images_public)?,
        };

        let allow = set("FETCH_ALLOW_NETS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|net| !net.is_empty())
            .map(|net| {
                net.parse()
                    .map_err(|err: String| anyhow::anyhow!("FETCH_ALLOW_NETS: {err}"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let client_ip_header = set("CLIENT_IP_HEADER")
            .map(|name| name.parse())
            .transpose()
            .context("CLIENT_IP_HEADER is not the name of a header")?;

        Ok(Self {
            listen_addr: set("LISTEN_ADDR").unwrap_or_else(|| LISTEN_ADDR.into()),
            static_dir: PathBuf::from(set("STATIC_DIR").unwrap_or_else(|| STATIC_DIR.into())),
            fetch: switch("FETCH", true)?,
            allow,
            client_ip_header,
            database_warn_bytes: status::warn_from(set("DATABASE_WARN_GB"))?,
            images: images_from(&set)?,
            switches,
        })
    }

    /// Where the image proxy keeps its images and its key, for a command that
    /// needs no more of the settings than that.
    pub fn images_from_env() -> anyhow::Result<images::Config> {
        images_from(&|name: &str| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        })
    }

    /// Says in the log what is set otherwise than rueder comes.
    pub fn log(&self) {
        let switches = &self.switches;
        let said = [
            (!self.fetch, "not fetching feeds (FETCH=off)"),
            (
                !switches.app_api,
                "apps of other makers cannot log in (APP_API=off)",
            ),
            (
                switches.content_reports,
                "readers may report feeds and articles (CONTENT_REPORTS=on)",
            ),
            (
                switches.images_public,
                "a cache in front of rueder may keep images (IMAGES_PUBLIC=on)",
            ),
            (
                !switches.reports,
                "taking no reports of errors from browsers (CLIENT_REPORTS=off)",
            ),
            (
                switches.admin_second_factor,
                "admins need a second factor (ADMIN_SECOND_FACTOR=required)",
            ),
        ];
        for (_, text) in said.iter().filter(|(set, _)| *set) {
            tracing::info!("{text}");
        }
    }
}

/// Where the image proxy keeps its images and its key, from what `set` gives.
fn images_from(set: &impl Fn(&str) -> Option<String>) -> anyhow::Result<images::Config> {
    let mut images =
        images::Config::new(set("IMAGE_CACHE_DIR").unwrap_or_else(|| IMAGE_CACHE_DIR.into()));
    if let Some(megabytes) = set("IMAGE_CACHE_MB") {
        let megabytes: u64 = megabytes
            .parse()
            .context("IMAGE_CACHE_MB is not a number")?;
        images.cache_bytes = megabytes.saturating_mul(1024 * 1024);
    }
    images.key = set("IMAGE_PROXY_KEY").map(String::into_bytes);
    Ok(images)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(pairs: &[(&str, &str)]) -> anyhow::Result<Settings> {
        Settings::from_vars(|name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        })
    }

    #[test]
    fn without_settings_the_server_is_as_rueder_comes() {
        let settings = settings(&[]).unwrap();
        assert_eq!(settings.switches, Switches::default());
        assert!(settings.fetch);
        assert_eq!(settings.listen_addr, LISTEN_ADDR);
        assert!(settings.allow.is_empty());
        assert_eq!(settings.client_ip_header, None);
        assert_eq!(settings.database_warn_bytes, None);
        assert_eq!(settings.images.key, None);
        // An empty line in .env is no setting.
        let empty = self::settings(&[("APP_API", ""), ("CLIENT_IP_HEADER", " ")]).unwrap();
        assert_eq!(empty.switches, Switches::default());
        assert_eq!(empty.client_ip_header, None);
    }

    #[test]
    fn a_switch_is_on_or_off_and_nothing_else() {
        let set = settings(&[
            ("APP_API", "off"),
            ("CONTENT_REPORTS", " on "),
            ("IMAGES_PUBLIC", "on"),
            ("CLIENT_REPORTS", "off"),
            ("ADMIN_SECOND_FACTOR", "required"),
            ("FETCH", "off"),
        ])
        .unwrap();
        assert_eq!(
            set.switches,
            Switches {
                reports: false,
                admin_second_factor: true,
                content_reports: true,
                app_api: false,
                images_public: true,
            }
        );
        assert!(!set.fetch);
        for (name, wrong) in [
            ("APP_API", "false"),
            ("IMAGES_PUBLIC", "yes"),
            ("FETCH", "0"),
            ("CONTENT_REPORTS", "On"),
            ("ADMIN_SECOND_FACTOR", "on"),
        ] {
            let error = settings(&[(name, wrong)]).unwrap_err().to_string();
            assert!(error.contains(name), "{error}");
        }
    }

    #[test]
    fn the_other_settings_are_checked_when_the_server_starts() {
        let set = settings(&[
            ("FETCH_ALLOW_NETS", "192.168.1.0/24, fd00::/8,"),
            ("CLIENT_IP_HEADER", "X-Forwarded-For"),
            ("DATABASE_WARN_GB", "50"),
            ("IMAGE_CACHE_MB", "10"),
            ("IMAGE_PROXY_KEY", "a key of the operator's own"),
            ("LISTEN_ADDR", "0.0.0.0:3000"),
        ])
        .unwrap();
        assert_eq!(set.allow.len(), 2);
        assert_eq!(set.client_ip_header.unwrap(), "x-forwarded-for");
        assert_eq!(set.database_warn_bytes, Some(50_000_000_000));
        assert_eq!(set.images.cache_bytes, 10 * 1024 * 1024);
        assert_eq!(
            set.images.key.as_deref(),
            Some(&b"a key of the operator's own"[..])
        );
        assert_eq!(set.listen_addr, "0.0.0.0:3000");
        for (name, wrong) in [
            ("FETCH_ALLOW_NETS", "intranet"),
            ("CLIENT_IP_HEADER", "not a header"),
            ("DATABASE_WARN_GB", "0"),
            ("IMAGE_CACHE_MB", "a lot"),
        ] {
            let error = settings(&[(name, wrong)]).unwrap_err();
            assert!(format!("{error:#}").contains(name), "{error:#}");
        }
    }
}
