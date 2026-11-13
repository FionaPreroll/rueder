//! Passkeys in Android apps: which apps may use the passkeys of rueder's site,
//! and the statement that says so to Android (Digital Asset Links).
//!
//! rueder's Android app shows the server's page in a WebView. Android gives
//! such an app the passkeys of a site only if the site names the app, with its
//! package and the fingerprint of the key it is signed with, at
//! `/.well-known/assetlinks.json` of the passkeys' host. The device then says
//! where the request came from not with the site's address but with the app's
//! key: `android:apk-key-hash:` and the SHA-256 of the signing certificate,
//! which [`crate::passkeys`] accepts as an origin for exactly these apps.
//!
//! The apps come from `PASSKEY_ANDROID_APPS`, entries of
//! `package=fingerprint` separated by commas. Without the setting it is the
//! app released on GitHub ([`OFFICIAL`]); `none` names none, and then the
//! statement is not there and no app's answer counts.

use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};

use crate::AppState;

/// The app released on GitHub (`app/`), with the fingerprint of the key
/// `app-release` in CI signs it with.
pub const OFFICIAL: &str = "io.github.fionapreroll.rueder=\
    CB:23:F2:B6:48:FD:07:44:2A:5C:46:D5:B9:81:67:D2:72:B2:90:55:C2:CA:8C:91:EF:42:2C:AA:06:28:15:25";

/// How many apps the setting may name.
const MAX_APPS: usize = 10;
/// The longest package name Android takes.
const MAX_PACKAGE: usize = 255;

/// An Android app that may use the passkeys: its package, and the SHA-256 of
/// the certificate it is signed with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidApp {
    package: String,
    fingerprint: [u8; 32],
}

impl AndroidApp {
    /// The apps `PASSKEY_ANDROID_APPS` names, or the official one where it is
    /// not set or empty. Anything it does not understand stops the start.
    pub fn from_setting(setting: Option<&str>) -> anyhow::Result<Vec<Self>> {
        let setting = setting.map(str::trim).unwrap_or_default();
        let setting = if setting.is_empty() {
            OFFICIAL
        } else {
            setting
        };
        if setting == "none" {
            return Ok(Vec::new());
        }
        let mut apps = Vec::new();
        for entry in setting.split(',') {
            let app = Self::parse(entry.trim()).map_err(|err| {
                anyhow::anyhow!("PASSKEY_ANDROID_APPS: {err}, in {:?}", entry.trim())
            })?;
            if !apps.contains(&app) {
                apps.push(app);
            }
        }
        anyhow::ensure!(
            apps.len() <= MAX_APPS,
            "PASSKEY_ANDROID_APPS names more than {MAX_APPS} apps"
        );
        Ok(apps)
    }

    /// One `package=fingerprint`, the fingerprint in hexadecimal pairs
    /// separated by colons, as `keytool` and `apksigner` write it.
    fn parse(entry: &str) -> anyhow::Result<Self> {
        let Some((package, fingerprint)) = entry.split_once('=') else {
            anyhow::bail!("an app is package=fingerprint");
        };
        let package = package.trim();
        anyhow::ensure!(is_package(package), "no Android package name");
        let pairs: Vec<&str> = fingerprint.trim().split(':').collect();
        anyhow::ensure!(
            pairs.len() == 32,
            "the fingerprint is the SHA-256 in 32 pairs, such as AB:CD:…"
        );
        let mut bytes = [0u8; 32];
        for (byte, pair) in bytes.iter_mut().zip(pairs) {
            anyhow::ensure!(
                pair.len() == 2 && pair.bytes().all(|c| c.is_ascii_hexdigit()),
                "the fingerprint is the SHA-256 in 32 pairs, such as AB:CD:…"
            );
            *byte = u8::from_str_radix(pair, 16)?;
        }
        Ok(Self {
            package: package.to_owned(),
            fingerprint: bytes,
        })
    }

    /// The app's package.
    pub fn package(&self) -> &str {
        &self.package
    }

    /// The origin the device names for this app in what it signs.
    pub fn origin(&self) -> String {
        format!(
            "android:apk-key-hash:{}",
            URL_SAFE_NO_PAD.encode(self.fingerprint)
        )
    }

    fn fingerprint_text(&self) -> String {
        self.fingerprint
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":")
    }
}

/// Whether `name` is a package name as Android takes it: at least two parts
/// separated by dots, each a letter followed by letters, digits or `_`.
fn is_package(name: &str) -> bool {
    let mut parts = 0;
    for part in name.split('.') {
        let mut chars = part.chars();
        let starts = chars.next().is_some_and(|c| c.is_ascii_alphabetic());
        if !starts || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
        parts += 1;
    }
    parts >= 2 && name.len() <= MAX_PACKAGE
}

/// The statement for these apps, one per package with all its keys.
pub fn statement(apps: &[AndroidApp]) -> Value {
    let mut packages: Vec<(&str, Vec<String>)> = Vec::new();
    for app in apps {
        let fingerprint = app.fingerprint_text();
        match packages.iter_mut().find(|(name, _)| *name == app.package) {
            Some((_, fingerprints)) => fingerprints.push(fingerprint),
            None => packages.push((&app.package, vec![fingerprint])),
        }
    }
    Value::Array(
        packages
            .into_iter()
            .map(|(package, fingerprints)| {
                json!({
                    "relation": [
                        "delegate_permission/common.handle_all_urls",
                        "delegate_permission/common.get_login_creds",
                    ],
                    "target": {
                        "namespace": "android_app",
                        "package_name": package,
                        "sha256_cert_fingerprints": fingerprints,
                    },
                })
            })
            .collect(),
    )
}

/// `GET /.well-known/assetlinks.json`, for anybody: Android reads it without a
/// session. Not there without passkeys or without apps.
pub async fn serve(State(state): State<AppState>) -> Response {
    let Some(passkeys) = &state.passkeys else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let apps = passkeys.android_apps();
    if apps.is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    (
        // Android looks again now and then; an hour keeps a change near.
        [(header::CACHE_CONTROL, "public, max-age=3600")],
        Json(statement(apps)),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_the_setting_it_is_the_official_app() {
        for setting in [None, Some(""), Some("  ")] {
            let apps = AndroidApp::from_setting(setting).unwrap();
            assert_eq!(apps.len(), 1);
            assert_eq!(apps[0].package, "io.github.fionapreroll.rueder");
            // The SHA-256 in base64url without padding, as Android writes it.
            let origin = apps[0].origin();
            let hash = origin.strip_prefix("android:apk-key-hash:").unwrap();
            assert_eq!(hash.len(), 43);
            let bytes = URL_SAFE_NO_PAD.decode(hash).unwrap();
            assert_eq!(bytes[..4], [0xCB, 0x23, 0xF2, 0xB6]);
            assert_eq!(bytes[28..], [0x06, 0x28, 0x15, 0x25]);
            assert_eq!(bytes, apps[0].fingerprint);
        }
        assert!(AndroidApp::from_setting(Some("none")).unwrap().is_empty());
    }

    #[test]
    fn the_setting_names_apps_with_their_keys() {
        let fingerprint = ["0a"; 32].join(":");
        let other = ["FF"; 32].join(":");
        let setting = format!(
            " org.example.reader={fingerprint} , org.example.reader = {other},\
             org.example.reader={fingerprint},com.example.b_2={other}"
        );
        let apps = AndroidApp::from_setting(Some(&setting)).unwrap();
        assert_eq!(apps.len(), 3);
        assert_eq!(apps[0].fingerprint, [0x0a; 32]);
        assert_eq!(
            statement(&apps),
            json!([
                {
                    "relation": [
                        "delegate_permission/common.handle_all_urls",
                        "delegate_permission/common.get_login_creds",
                    ],
                    "target": {
                        "namespace": "android_app",
                        "package_name": "org.example.reader",
                        "sha256_cert_fingerprints": [(["0A"; 32].join(":")), other.clone()],
                    },
                },
                {
                    "relation": [
                        "delegate_permission/common.handle_all_urls",
                        "delegate_permission/common.get_login_creds",
                    ],
                    "target": {
                        "namespace": "android_app",
                        "package_name": "com.example.b_2",
                        "sha256_cert_fingerprints": [other],
                    },
                },
            ])
        );
    }

    #[test]
    fn what_is_no_app_stops_the_start() {
        let good = ["AB"; 32].join(":");
        let long = format!("a.{}", "b".repeat(MAX_PACKAGE));
        let many = vec![format!("a.b={good}"); 1]
            .into_iter()
            .chain((0..MAX_APPS).map(|n| format!("a.b{n}={good}")))
            .collect::<Vec<_>>()
            .join(",");
        for bad in [
            "a.b".to_owned(),
            format!("={good}"),
            format!("single={good}"),
            format!("a..b={good}"),
            format!("1a.b={good}"),
            format!("a.b-c={good}"),
            format!("{long}={good}"),
            "a.b=".to_owned(),
            format!("a.b={}", ["AB"; 31].join(":")),
            format!("a.b={}", ["AB"; 33].join(":")),
            format!("a.b={}", good.replace(':', "")),
            format!("a.b={}", good.replacen("AB", "GG", 1)),
            format!("a.b={}", good.replacen("AB", "+1", 1)),
            format!("a.b={}", good.replacen("AB", "ABC", 1)),
            format!("a.b={good},"),
            format!("none,a.b={good}"),
            many,
        ] {
            assert!(AndroidApp::from_setting(Some(&bad)).is_err(), "{bad}");
        }
    }
}
