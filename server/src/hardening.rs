//! What every answer and every changing request has to pass, whatever the
//! endpoint: the headers that tell the browser what a page of rueder may do,
//! and the check that a change was asked for by rueder's own pages.
//!
//! - [`headers`] gives every answer its Content-Security-Policy and the other
//!   headers. Article HTML is cleaned before it is stored; the policy is the
//!   second line, for the day the cleaning lets something through: no script
//!   runs but the ones rueder ships, and a page loads images, styles and data
//!   from rueder alone.
//! - [`same_origin`] refuses a request that changes something and was sent by
//!   a page of another site. The session cookie is `SameSite=Lax` already;
//!   this is the second line against cross-site request forgery.

use std::{fs, path::Path, sync::Arc};

use axum::{
    extract::{Request, State},
    http::{
        HeaderMap, HeaderName, HeaderValue, Method, StatusCode,
        header::{self, Entry},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};
use url::Url;

/// What an answer of the API or the image proxy may do if a browser shows it
/// as a page: nothing. The image proxy sets its own, with `sandbox` on top.
const DATA_POLICY: &str = "default-src 'none'; frame-ancestors 'none'";

/// How many inline scripts of the frontend's page get a place in the policy,
/// and how much of the page is looked at for them. SvelteKit's page has two
/// and a few kilobytes.
const MAX_INLINE_SCRIPTS: usize = 8;
const MAX_PAGE: u64 = 1024 * 1024;

/// Where SvelteKit's own scripts lie in the build, how many of them are looked
/// at for the style of its announcer, and how large one may be.
const SCRIPTS: &str = "_app/immutable";
const MAX_SCRIPTS: usize = 500;
const MAX_SCRIPT: u64 = 4 * 1024 * 1024;
/// How many styles of the announcer get a place in the policy: one, in a build
/// that is not broken.
const MAX_ANNOUNCER_STYLES: usize = 4;

/// The headers every answer gets, unless the endpoint set its own.
const ALWAYS: [(HeaderName, &str); 7] = [
    // The type the server names is the type, whatever the bytes look like.
    (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
    // Browsers that were here once come back over HTTPS only, for a year. It
    // holds for this host alone, which needs HTTPS anyway: the session cookie
    // is `Secure`. Browsers take it only from an answer over HTTPS, so plain
    // HTTP on localhost stays as it is. Set here because not every proxy in
    // front can set it (`tailscale serve` cannot).
    (header::STRICT_TRANSPORT_SECURITY, "max-age=31536000"),
    // Sites that are linked from articles do not learn where the reader
    // came from.
    (header::REFERRER_POLICY, "no-referrer"),
    // No other site shows rueder in a frame, for browsers that do not know
    // `frame-ancestors`.
    (header::X_FRAME_OPTIONS, "DENY"),
    // A page that rueder opens, or that opens rueder, gets no handle on its
    // window, and no other site loads what rueder serves.
    (
        HeaderName::from_static("cross-origin-opener-policy"),
        "same-origin",
    ),
    (
        HeaderName::from_static("cross-origin-resource-policy"),
        "same-origin",
    ),
    // A reader needs none of these, so nothing an article brings gets them.
    (
        HeaderName::from_static("permissions-policy"),
        "camera=(), display-capture=(), geolocation=(), microphone=(), payment=(), usb=()",
    ),
];

/// The Content-Security-Policy of the frontend's pages.
///
/// Scripts and styles come from rueder's own files only. SvelteKit starts the
/// app with a script inside the page, and the page sets the theme with another
/// before the first paint; those two are allowed by their hash, so no
/// `'unsafe-inline'` is needed and a script in an article's text never runs.
/// Images come from the image proxy; `data:` is for the icon in the page.
/// Audio and video are the one thing a browser fetches from elsewhere, and
/// only once the reader starts them.
///
/// No `style` attribute is allowed but the one SvelteKit gives the element it
/// hides for screen readers (`#svelte-announcer`), by the hash of exactly its
/// value: `'unsafe-hashes'` lets nothing else through.
fn page_policy(script_hashes: &[String], style_attr_hashes: &[String]) -> String {
    let mut scripts = String::from("script-src 'self'");
    for hash in script_hashes {
        scripts.push_str(&format!(" 'sha256-{hash}'"));
    }
    let mut style_attrs = String::from("style-src-attr 'unsafe-hashes'");
    for hash in style_attr_hashes {
        style_attrs.push_str(&format!(" 'sha256-{hash}'"));
    }
    [
        "default-src 'none'",
        &scripts,
        "style-src 'self'",
        if style_attr_hashes.is_empty() {
            ""
        } else {
            &style_attrs
        },
        "img-src 'self' data:",
        "font-src 'self'",
        "connect-src 'self'",
        "media-src http: https:",
        "manifest-src 'self'",
        "base-uri 'none'",
        "form-action 'self'",
        "frame-ancestors 'none'",
    ]
    .into_iter()
    .filter(|directive| !directive.is_empty())
    .collect::<Vec<_>>()
    .join("; ")
}

/// The SHA-256 of the `style` attribute SvelteKit gives its announcer, as it
/// stands in one of its scripts. The script is rueder's own build; this only
/// looks for the one attribute after the element's id.
pub fn announcer_styles(script: &str) -> Vec<String> {
    const ID: &str = "svelte-announcer";
    const STYLE: &str = "style=\"";
    /// How far after the id the attribute may stand.
    const NEAR: usize = 300;
    let mut hashes = Vec::new();
    let mut at = 0;
    while hashes.len() < MAX_ANNOUNCER_STYLES {
        let Some(id) = script[at..].find(ID).map(|found| at + found) else {
            break;
        };
        at = id + ID.len();
        let mut end = script.len().min(at + NEAR);
        while !script.is_char_boundary(end) {
            end -= 1;
        }
        let window = &script[at..end];
        let Some(start) = window.find(STYLE).map(|found| found + STYLE.len()) else {
            continue;
        };
        let Some(length) = window[start..].find('"') else {
            continue;
        };
        let style = &window[start..start + length];
        let hash = STANDARD.encode(Sha256::digest(style.as_bytes()));
        if !hashes.contains(&hash) {
            hashes.push(hash);
        }
    }
    hashes
}

/// The styles of the announcer in the scripts of a build, read from its
/// `_app/immutable` folder within limits for how many files and how large.
fn announcer_styles_in(static_dir: &Path) -> Vec<String> {
    let mut hashes = Vec::new();
    let mut folders = vec![static_dir.join(SCRIPTS)];
    let mut read = 0;
    while let Some(folder) = folders.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                folders.push(path);
                continue;
            }
            if !kind.is_file() || path.extension().is_none_or(|ext| ext != "js") {
                continue;
            }
            read += 1;
            if read > MAX_SCRIPTS {
                return hashes;
            }
            let small = entry.metadata().is_ok_and(|meta| meta.len() <= MAX_SCRIPT);
            let Some(script) = small.then(|| fs::read_to_string(&path).ok()).flatten() else {
                continue;
            };
            for hash in announcer_styles(&script) {
                if hashes.len() < MAX_ANNOUNCER_STYLES && !hashes.contains(&hash) {
                    hashes.push(hash);
                }
            }
        }
    }
    hashes
}

/// The SHA-256 of every script that stands inside a page, as the policy
/// names them. The page is rueder's own, built by SvelteKit; this is not a
/// parser for HTML somebody else wrote. A script it misses does not run.
pub fn inline_scripts(page: &str) -> Vec<String> {
    let lower = page.to_ascii_lowercase();
    let mut hashes = Vec::new();
    let mut at = 0;
    while hashes.len() < MAX_INLINE_SCRIPTS {
        let Some(tag) = lower[at..].find("<script").map(|found| at + found) else {
            break;
        };
        let Some(open_end) = lower[tag..].find('>').map(|found| tag + found) else {
            break;
        };
        let Some(close) = lower[open_end..]
            .find("</script")
            .map(|found| open_end + found)
        else {
            break;
        };
        // A script with an address is allowed by where it comes from.
        if !lower[tag..open_end].contains("src=") {
            let script = &page[open_end + 1..close];
            hashes.push(STANDARD.encode(Sha256::digest(script.as_bytes())));
        }
        at = close;
    }
    hashes
}

/// The policies of a server, worked out once when it starts.
#[derive(Clone)]
pub struct Policies {
    page: HeaderValue,
}

impl Policies {
    /// For the frontend in this directory. Its `index.html` and its scripts
    /// are read once: a new build of the frontend needs a restart of the
    /// server.
    pub fn for_frontend(static_dir: &Path) -> Self {
        let index = static_dir.join("index.html");
        let page = fs::metadata(&index)
            .ok()
            .filter(|file| file.len() <= MAX_PAGE)
            .and_then(|_| fs::read_to_string(&index).ok());
        if page.is_none() {
            tracing::warn!(
                "no frontend at {}: its scripts will not run",
                index.display()
            );
        }
        Self::new(
            page.as_deref().unwrap_or_default(),
            &announcer_styles_in(static_dir),
        )
    }

    pub fn for_page(page: &str) -> Self {
        Self::new(page, &[])
    }

    fn new(page: &str, style_attr_hashes: &[String]) -> Self {
        let policy = page_policy(&inline_scripts(page), style_attr_hashes);
        Self {
            // Hashes in base64 and fixed words: always a header's value.
            page: HeaderValue::from_str(&policy)
                .unwrap_or(HeaderValue::from_static("default-src 'none'")),
        }
    }
}

/// Whether the path belongs to the API or the image proxy, whose answers are
/// data and never a page.
fn is_data(path: &str) -> bool {
    path == "/api" || path.starts_with("/api/") || path.starts_with("/img/")
}

/// Around everything the server answers: the headers of [`ALWAYS`] and a
/// Content-Security-Policy. What the API answers is one account's own and
/// is kept by no cache.
pub async fn headers(
    State(policies): State<Arc<Policies>>,
    request: Request,
    next: Next,
) -> Response {
    let data = is_data(request.uri().path());
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    for (name, value) in ALWAYS {
        if let Entry::Vacant(entry) = headers.entry(name) {
            entry.insert(HeaderValue::from_static(value));
        }
    }
    if let Entry::Vacant(entry) = headers.entry(header::CONTENT_SECURITY_POLICY) {
        entry.insert(if data {
            HeaderValue::from_static(DATA_POLICY)
        } else {
            policies.page.clone()
        });
    }
    if data && let Entry::Vacant(entry) = headers.entry(header::CACHE_CONTROL) {
        entry.insert(HeaderValue::from_static("no-store"));
    }
    response
}

/// The host a request was sent to, in lower case.
fn host(request: &Request) -> Option<String> {
    let named = request
        .headers()
        .get(header::HOST)
        .and_then(|host| host.to_str().ok());
    // HTTP/2 names it in the address instead.
    let host = named.or_else(|| request.uri().authority().map(|host| host.as_str()))?;
    Some(host.to_ascii_lowercase())
}

/// Whether the `Origin` a browser named is the host the request was sent to.
/// Behind a reverse proxy that is the proxy's name, as long as the proxy
/// hands the `Host` header on.
fn is_own(origin: &str, host: &str) -> bool {
    let Ok(origin) = Url::parse(origin) else {
        return false;
    };
    let Some(name) = origin.host_str() else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    // `Url` leaves out the port a scheme has anyway; a `Host` may name it.
    match (origin.port(), origin.port_or_known_default()) {
        (Some(port), _) => host == format!("{name}:{port}"),
        (None, Some(port)) => host == name || host == format!("{name}:{port}"),
        (None, None) => host == name,
    }
}

/// Whether a request that changes something may pass: not if a browser says
/// it comes from a page of another site.
///
/// Browsers say so in `Sec-Fetch-Site`, which a page cannot set or change;
/// older ones only name the page's `Origin`, which has to be this server
/// then. A request with neither comes from no browser, such as a script with
/// the session's cookie in hand, and no page of another site can send one.
fn allowed(request: &Request) -> bool {
    let headers: &HeaderMap = request.headers();
    let named = |name: HeaderName| headers.get(name).map(|value| value.to_str().ok());
    match named(HeaderName::from_static("sec-fetch-site")) {
        // `none` is what the reader did by hand, such as typing an address.
        Some(Some("same-origin" | "none")) => return true,
        Some(_) => return false,
        None => {}
    }
    match named(header::ORIGIN) {
        None => true,
        Some(Some(origin)) => host(request).is_some_and(|host| is_own(origin, &host)),
        Some(None) => false,
    }
}

/// Around everything: a request that changes something is refused if it
/// comes from a page of another site.
pub async fn same_origin(request: Request, next: Next) -> Response {
    let reads_only = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    if reads_only || allowed(&request) {
        return next.run(request).await;
    }
    tracing::debug!("refused a request from another site: {}", request.uri());
    (
        StatusCode::FORBIDDEN,
        "this request comes from another site",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_inside_a_page_are_found() {
        let page = "<!doctype html><head>\n\
            <script src=\"/a.js\"></script>\n\
            <SCRIPT>one()</SCRIPT>\n\
            <script type=\"module\" data-x>\n\ttwo();\n</script>\n\
            </head><body><script>";
        let hash = |script: &str| STANDARD.encode(Sha256::digest(script.as_bytes()));
        assert_eq!(inline_scripts(page), [hash("one()"), hash("\n\ttwo();\n")]);
        // The hash of `alert(1)`, as browsers and the specification have it.
        assert_eq!(
            inline_scripts("<script>alert(1)</script>"),
            ["bhHHL3z2vDgxUt0W3dWQOrprscmda2Y5pLsLg4GF+pI="]
        );
        assert!(inline_scripts("").is_empty());
        assert!(inline_scripts("<script>never closed").is_empty());
        assert!(inline_scripts("<script").is_empty());
        let many = "<script>x</script>".repeat(MAX_INLINE_SCRIPTS + 5);
        assert_eq!(inline_scripts(&many).len(), MAX_INLINE_SCRIPTS);
        // Not at a character's border, and still no panic.
        assert_eq!(inline_scripts("ä<script>ö</script>ü").len(), 1);
    }

    #[test]
    fn the_policy_of_a_page_names_its_scripts() {
        let policies = Policies::for_page("<script>start()</script>");
        let policy = policies.page.to_str().unwrap();
        assert!(policy.starts_with("default-src 'none'; script-src 'self' 'sha256-"));
        assert!(!policy.contains("unsafe"));
        let none = Policies::for_page("");
        assert!(none.page.to_str().unwrap().contains("script-src 'self';"));
    }

    /// The attribute as SvelteKit writes it, and its hash, which Firefox and
    /// Chromium name when they block it.
    const ANNOUNCER: &str = r#"<div id="svelte-announcer" aria-live="assertive" aria-atomic="true" style="position: absolute; left: 0; top: 0; clip: rect(0 0 0 0); clip-path: inset(50%); overflow: hidden; white-space: nowrap; width: 1px; height: 1px"></div>"#;
    const ANNOUNCER_HASH: &str = "S8qMpvofolR8Mpjy4kQvEm7m1q8clzU4dfDH0AmvZjo=";

    #[test]
    fn the_style_of_the_announcer_is_allowed_by_its_hash_alone() {
        let script = format!("x=1;{ANNOUNCER};y=2;{ANNOUNCER}");
        assert_eq!(announcer_styles(&script), [ANNOUNCER_HASH]);
        // Another element's style, or one far from the id, is not it.
        assert!(announcer_styles(r#"<div id="other" style="color: red"></div>"#).is_empty());
        let far = format!("svelte-announcer{}style=\"color: red\"", " ".repeat(400));
        assert!(announcer_styles(&far).is_empty());
        assert!(announcer_styles("svelte-announcer style=\"never closed").is_empty());
        assert_eq!(
            announcer_styles("ä svelte-announcer ö style=\"ü\"").len(),
            1
        );
        // Not at a character's border, and still no panic.
        let near_end = format!("svelte-announcer{}", "ä".repeat(200));
        assert!(announcer_styles(&near_end).is_empty());
        let many: String = (0..10)
            .map(|n| format!("svelte-announcer style=\"top: {n}px\" "))
            .collect();
        assert_eq!(announcer_styles(&many).len(), MAX_ANNOUNCER_STYLES);

        let policies = Policies::new("", &announcer_styles(ANNOUNCER));
        let policy = policies.page.to_str().unwrap();
        assert!(policy.contains(&format!(
            "; style-src 'self'; style-src-attr 'unsafe-hashes' 'sha256-{ANNOUNCER_HASH}'; "
        )));
        // Without it, no attribute at all.
        let none = Policies::for_page("");
        let none = none.page.to_str().unwrap();
        assert!(!none.contains("style-src-attr"));
        assert!(!none.contains("unsafe"));
        assert!(!none.contains("; ;"));
    }

    #[test]
    fn the_announcer_is_found_in_the_scripts_of_a_build() {
        let dir = std::env::temp_dir().join(format!("rueder-hardening-{}", std::process::id()));
        let chunks = dir.join("_app/immutable/chunks");
        fs::create_dir_all(&chunks).unwrap();
        fs::write(dir.join("index.html"), "<script>start()</script>").unwrap();
        fs::write(chunks.join("a.js"), "nothing here").unwrap();
        fs::write(chunks.join("b.js"), format!("var t=`{ANNOUNCER}`")).unwrap();
        fs::write(chunks.join("c.css"), ANNOUNCER.replace("absolute", "fixed")).unwrap();
        let policies = Policies::for_frontend(&dir);
        let policy = policies.page.to_str().unwrap().to_owned();
        fs::remove_dir_all(&dir).unwrap();
        assert!(
            policy.contains(&format!("'sha256-{ANNOUNCER_HASH}'")),
            "{policy}"
        );
        assert_eq!(policy.matches("sha256-").count(), 2, "{policy}");
    }

    #[test]
    fn an_origin_is_the_host_or_not() {
        assert!(is_own("https://reader.example", "reader.example"));
        assert!(is_own("https://Reader.Example", "reader.example"));
        assert!(is_own("https://reader.example", "reader.example:443"));
        assert!(is_own("http://reader.example", "reader.example:80"));
        assert!(is_own("http://localhost:3000", "localhost:3000"));
        assert!(is_own("http://[::1]:3000", "[::1]:3000"));
        assert!(!is_own("https://reader.example", "reader.example:8443"));
        assert!(!is_own("https://reader.example:8443", "reader.example"));
        assert!(!is_own("http://localhost:3000", "localhost:3001"));
        assert!(!is_own("https://evil.example", "reader.example"));
        assert!(!is_own(
            "https://reader.example.evil.example",
            "reader.example"
        ));
        assert!(!is_own(
            "https://evil.example/reader.example",
            "reader.example"
        ));
        assert!(!is_own(
            "https://reader.example@evil.example",
            "reader.example"
        ));
        assert!(!is_own("null", "reader.example"));
        assert!(!is_own("", "reader.example"));
        assert!(!is_own("reader.example", "reader.example"));
    }
}
