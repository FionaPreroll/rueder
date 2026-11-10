//! The image proxy: which addresses it answers, what it lets through, what it
//! keeps, and what a source gets to see.

use std::{
    collections::HashMap,
    io::Cursor,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use http_body_util::BodyExt;
use image::{DynamicImage, GenericImageView, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use rueder::{
    AppState,
    cleaning::{Base, Rules, clean},
    http_client::{self, HttpClient},
    images::{Cache, Config, Format, ImageProxy, Picture, Refused, Signer, Size, fit},
    router,
};
use sqlx::PgPool;
use tower::ServiceExt;

const KEY: &[u8] = b"a key for tests, long enough to be one";

/// A picture with something in it, so it does not shrink to nothing as a file.
fn picture(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let image = RgbImage::from_fn(width, height, |x, y| {
        Rgb([(x % 251) as u8, (y % 241) as u8, ((x * y) % 239) as u8])
    });
    let mut out = Vec::new();
    DynamicImage::from(image)
        .write_to(&mut Cursor::new(&mut out), format)
        .unwrap();
    out
}

fn see_through(width: u32, height: u32) -> Vec<u8> {
    let image = RgbaImage::from_fn(width, height, |x, y| {
        // The left half lets everything through, the right half nothing.
        Rgba([
            (x % 251) as u8,
            (y % 241) as u8,
            7,
            if x < width / 2 { 0 } else { 255 },
        ])
    });
    let mut out = Vec::new();
    DynamicImage::from(image)
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .unwrap();
    out
}

fn dimensions(picture: &Picture) -> (u32, u32) {
    image::load_from_memory(&picture.bytes)
        .unwrap()
        .dimensions()
}

/// A directory of its own for a test's cache.
fn cache_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rueder-images-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn only_the_server_makes_addresses() {
    let signer = Signer::new(KEY.to_vec()).unwrap();
    let address = signer
        .address(Size::Image, "https://example.org/a.png?x=1&y=2")
        .unwrap();
    let parts: Vec<&str> = address.split('/').collect();
    assert_eq!(parts[..2], ["", "img"]);
    assert_eq!((parts[2].len(), parts[3], parts.len()), (22, "image", 5));
    // Nothing in an address needs escaping, in a path or in an attribute.
    assert!(
        address
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/-_".contains(c))
    );
    // The same image in another size is another address.
    let thumbnail = signer
        .address(Size::Thumbnail, "https://example.org/a.png?x=1&y=2")
        .unwrap();
    assert_ne!(thumbnail.split('/').nth(2), address.split('/').nth(2));

    // Only what a browser may be sent to fetch.
    for no_address in [
        "",
        "a.png",
        "/api/logout",
        "//example.org/a.png",
        "javascript:alert(1)",
        "data:image/png;base64,AAAA",
        "ftp://example.org/a.png",
        "file:///etc/passwd",
        "https://",
    ] {
        assert_eq!(
            signer.address(Size::Image, no_address),
            None,
            "{no_address}"
        );
    }
    let long = format!("https://example.org/{}", "a".repeat(2000));
    assert_eq!(signer.address(Size::Image, &long), None);
    // A key that is guessed quickly is none.
    assert!(Signer::new(b"short".to_vec()).is_err());
}

#[test]
fn images_pass_and_nothing_else() {
    // What fits passes as it came.
    for (format, kind) in [
        (ImageFormat::Png, Format::Png),
        (ImageFormat::Jpeg, Format::Jpeg),
        (ImageFormat::Gif, Format::Gif),
        (ImageFormat::WebP, Format::WebP),
    ] {
        let bytes = picture(300, 200, format);
        let passed = fit(bytes.clone(), Size::Image).unwrap();
        assert_eq!(
            (passed.format, passed.format.mime().len() > 8),
            (kind, true)
        );
        assert_eq!(passed.bytes, bytes, "{format:?}");
    }

    // The first bytes of a GIF and then a script: its head says nonsense about
    // its size, and it is refused for that.
    assert!(fit(b"GIF89a<script>alert(1)</script>".to_vec(), Size::Image).is_err());

    // Whatever calls itself an image and is none is refused: told by the
    // first bytes, not by a name or a header.
    let bmp = picture(10, 10, ImageFormat::Png).len();
    assert!(bmp > 0);
    for not_an_image in [
        &b""[..],
        b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>",
        b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"/>",
        b"<!DOCTYPE html><html><body><img src=x onerror=alert(1)></body></html>",
        b"\x89PNG\r\n\x1a\nnot really a picture at all",
        b"\xff\xd8\xff\xe0 the start of a JPEG and then nothing",
        b"BM\x00\x00\x00\x00 a bitmap, which the proxy does not take",
        b"RIFF\x00\x00\x00\x00WAVEfmt  a sound in the wrapping of WebP",
        b"\x00\x00\x01\x00\x01\x00\x10\x10 an icon file",
    ] {
        assert_eq!(
            fit(not_an_image.to_vec(), Size::Image),
            Err(Refused::NotAnImage),
            "{:?}",
            String::from_utf8_lossy(not_an_image)
        );
    }
}

#[test]
fn large_images_are_made_to_fit() {
    // A photo: smaller, in the same shape, as a JPEG.
    let photo = picture(3000, 1500, ImageFormat::Jpeg);
    let fitted = fit(photo.clone(), Size::Image).unwrap();
    assert_eq!(fitted.format, Format::Jpeg);
    assert_eq!(dimensions(&fitted), (2000, 1000));
    assert!(fitted.bytes.len() < photo.len());

    // The same source in the other sizes, upright as well as lying.
    let tall = picture(600, 2400, ImageFormat::Png);
    assert_eq!(
        dimensions(&fit(tall.clone(), Size::Thumbnail).unwrap()),
        (200, 800)
    );
    assert_eq!(
        dimensions(&fit(tall.clone(), Size::Icon).unwrap()),
        (64, 256)
    );
    assert_eq!(dimensions(&fit(tall, Size::Image).unwrap()), (500, 2000));
    // One pixel too wide is too wide.
    let barely = fit(picture(801, 10, ImageFormat::Gif), Size::Thumbnail).unwrap();
    assert_eq!(dimensions(&barely).0, 800);
    assert_eq!(barely.format, Format::Jpeg);

    // What one can see through stays so, which a JPEG could not do.
    let logo = fit(see_through(1200, 300), Size::Icon).unwrap();
    assert_eq!(logo.format, Format::Png);
    assert_eq!(dimensions(&logo), (256, 64));
    let pixel = image::load_from_memory(&logo.bytes)
        .unwrap()
        .get_pixel(0, 0);
    assert_eq!(pixel.0[3], 0);
}

#[test]
fn a_small_file_that_unpacks_into_a_huge_image_is_refused() {
    // 49 million pixels of nothing pack into a few kilobytes.
    let mut bomb = Vec::new();
    DynamicImage::from(image::GrayImage::new(7000, 7000))
        .write_to(&mut Cursor::new(&mut bomb), ImageFormat::Png)
        .unwrap();
    assert!(bomb.len() < 1_000_000, "{}", bomb.len());
    let started = Instant::now();
    assert_eq!(fit(bomb, Size::Thumbnail), Err(Refused::TooLarge));
    // Refused by what the file says about itself, before anything is unpacked.
    assert!(started.elapsed() < Duration::from_millis(500));

    // Just under the limit is an image like any other.
    let mut large = Vec::new();
    DynamicImage::from(image::GrayImage::new(6000, 4990))
        .write_to(&mut Cursor::new(&mut large), ImageFormat::Png)
        .unwrap();
    assert_eq!(
        dimensions(&fit(large, Size::Thumbnail).unwrap()),
        (800, 665)
    );
}

#[test]
fn the_cache_keeps_what_was_used_last() {
    let dir = cache_dir("cache");
    let cache = Cache::open(&dir, 10_000).unwrap();
    let image = |n: u8| Picture {
        format: Format::Png,
        bytes: vec![n; 2999],
    };
    let url = |n: u8| format!("https://example.org/{n}.png");
    assert_eq!(cache.get(Size::Image, &url(1)), None);
    for n in 1..=3 {
        cache.put(Size::Image, &url(n), &image(n));
    }
    assert_eq!(cache.used(), 9000);
    assert_eq!(cache.get(Size::Image, &url(2)), Some(image(2)));
    // The same source in another size is another image.
    assert_eq!(cache.get(Size::Thumbnail, &url(2)), None);

    // Make the files as old as they are numbered, then use the oldest again.
    let age = |n: u8, seconds: u64| {
        let shard = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .find_map(|shard| {
                let files = std::fs::read_dir(shard.path()).ok()?;
                files
                    .flatten()
                    .find(|file| std::fs::read(file.path()).is_ok_and(|bytes| bytes[1] == n))
            });
        let file = std::fs::File::options()
            .write(true)
            .open(shard.unwrap().path())
            .unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(seconds))
            .unwrap();
    };
    age(1, 300);
    age(2, 200);
    age(3, 100);
    assert_eq!(cache.get(Size::Image, &url(1)), Some(image(1)));

    // One more is over the limit: the ones unused longest go, down to four
    // fifths of the limit.
    cache.put(Size::Image, &url(4), &image(4));
    assert_eq!(cache.get(Size::Image, &url(2)), None);
    assert_eq!(cache.get(Size::Image, &url(3)), None);
    assert_eq!(cache.get(Size::Image, &url(1)), Some(image(1)));
    assert_eq!(cache.get(Size::Image, &url(4)), Some(image(4)));
    assert_eq!(cache.used(), 6000);

    // What a write that broke off left behind goes too, once it is old.
    let shard = std::fs::read_dir(&dir).unwrap().flatten().next().unwrap();
    let partial = shard.path().join("0123456789abcdef.partial");
    std::fs::write(&partial, [0; 100]).unwrap();
    cache.sweep();
    assert!(partial.exists());
    std::fs::File::options()
        .write(true)
        .open(&partial)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(2 * 60 * 60))
        .unwrap();
    cache.sweep();
    assert!(!partial.exists());

    // A file that is not what the cache wrote is no image.
    let odd = Cache::open(&dir, 10_000).unwrap();
    assert_eq!(odd.used(), 6000);
    let path = std::fs::read_dir(shard.path()).unwrap().flatten().next();
    if let Some(file) = path {
        std::fs::write(file.path(), [9, 9, 9]).unwrap();
        let n = [1, 4]
            .into_iter()
            .filter(|n| odd.get(Size::Image, &url(*n)).is_none())
            .count();
        assert_eq!(n, 1);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// A site with images, and what it was asked.
#[derive(Default)]
struct Site {
    files: HashMap<String, (StatusCode, &'static str, Vec<u8>)>,
    asked: Vec<(String, HeaderMap)>,
}

type Shared = Arc<Mutex<Site>>;

async fn file(
    State(site): State<Shared>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> Response {
    let mut site = site.lock().unwrap();
    site.asked.push((name.clone(), headers));
    match site.files.get(&name) {
        Some((status, kind, bytes)) => {
            (*status, [(header::CONTENT_TYPE, *kind)], bytes.clone()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Starts the site and returns where it is.
async fn site(files: &[(&str, StatusCode, &'static str, Vec<u8>)]) -> (String, Shared) {
    let site: Shared = Arc::default();
    for (name, status, kind, bytes) in files {
        let file = (*status, *kind, bytes.clone());
        site.lock().unwrap().files.insert((*name).to_owned(), file);
    }
    let router = Router::new()
        .route("/{name}", get(file))
        .with_state(site.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("http://{addr}"), site)
}

/// A proxy that may reach the site on this machine, or may not.
fn proxy(name: &str, local: bool) -> Arc<ImageProxy> {
    let allow = if local {
        vec!["127.0.0.1".parse().unwrap()]
    } else {
        Vec::new()
    };
    let client = HttpClient::new(http_client::Config {
        allow,
        ..Default::default()
    })
    .unwrap();
    let config = Config {
        key: Some(KEY.to_vec()),
        max_source: 300_000,
        ..Config::new(cache_dir(name))
    };
    Arc::new(ImageProxy::open(config, client).unwrap())
}

async fn get_path(state: AppState, path: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    let app = router(state, "missing".as_ref());
    let response = app
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, body.to_vec())
}

fn asked(site: &Shared, name: &str) -> usize {
    let site = site.lock().unwrap();
    site.asked.iter().filter(|(file, _)| file == name).count()
}

#[sqlx::test]
async fn an_image_is_fetched_once_and_served_as_an_image(db: PgPool) {
    let small = picture(40, 30, ImageFormat::Png);
    let (at, site) = site(&[
        ("a.png", StatusCode::OK, "text/html", small.clone()),
        (
            "photo.jpg",
            StatusCode::OK,
            "image/jpeg",
            picture(1000, 500, ImageFormat::Jpeg),
        ),
    ])
    .await;
    let images = proxy("served", true);
    let state = AppState::new(db).with_images(images.clone());
    let address = images
        .signer()
        .address(Size::Image, &format!("{at}/a.png"))
        .unwrap();

    let (status, headers, body) = get_path(state.clone(), &address).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, small);
    let said = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    // What it is comes from its bytes, not from what the source called it,
    // and the browser is told not to think again.
    assert_eq!(said("content-type"), Some("image/png"));
    assert_eq!(said("x-content-type-options"), Some("nosniff"));
    assert_eq!(
        said("content-security-policy"),
        Some("default-src 'none'; sandbox")
    );
    assert_eq!(said("cross-origin-resource-policy"), Some("same-origin"));
    assert_eq!(
        said("cache-control"),
        Some("private, max-age=31536000, immutable")
    );

    // The second time it comes from the cache.
    let (status, _, body) = get_path(state.clone(), &address).await;
    assert_eq!((status, body), (StatusCode::OK, small));
    assert_eq!(asked(&site, "a.png"), 1);

    // The source learns nothing about the reader.
    {
        let site = site.lock().unwrap();
        let (_, headers) = &site.asked[0];
        assert_eq!(headers.get("cookie"), None);
        assert_eq!(headers.get("referer"), None);
        let agent = headers.get("user-agent").unwrap().to_str().unwrap();
        assert!(agent.starts_with("rueder/"), "{agent}");
    }

    // The same photo as a thumbnail is fetched and kept on its own, smaller.
    let photo = format!("{at}/photo.jpg");
    let thumbnail = images.signer().address(Size::Thumbnail, &photo).unwrap();
    let (status, headers, body) = get_path(state.clone(), &thumbnail).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get("content-type").unwrap(), "image/jpeg");
    assert_eq!(
        image::load_from_memory(&body).unwrap().dimensions(),
        (800, 400)
    );
    let whole = images.signer().address(Size::Image, &photo).unwrap();
    let (_, _, body) = get_path(state.clone(), &whole).await;
    assert_eq!(
        image::load_from_memory(&body).unwrap().dimensions(),
        (1000, 500)
    );
    assert_eq!(asked(&site, "photo.jpg"), 2);
    assert!(images.cache().used() > 0);
}

#[sqlx::test]
async fn an_address_the_server_did_not_make_is_not_answered(db: PgPool) {
    let (at, site) = site(&[
        (
            "a.png",
            StatusCode::OK,
            "image/png",
            picture(40, 30, ImageFormat::Png),
        ),
        (
            "b.png",
            StatusCode::OK,
            "image/png",
            picture(40, 30, ImageFormat::Png),
        ),
    ])
    .await;
    let images = proxy("unsigned", true);
    let state = AppState::new(db.clone()).with_images(images.clone());
    let signer = images.signer();
    let a = signer.address(Size::Image, &format!("{at}/a.png")).unwrap();
    let b = signer.address(Size::Image, &format!("{at}/b.png")).unwrap();
    let part = |address: &str, n: usize| address.split('/').nth(n).unwrap().to_owned();

    let mut flipped = part(&a, 2);
    let last = if flipped.ends_with('A') { 'B' } else { 'A' };
    flipped.pop();
    flipped.push(last);
    let other_key = Signer::new(b"another key, just as long as the first".to_vec()).unwrap();
    for forged in [
        // The signature of one image with the address of another.
        format!("/img/{}/image/{}", part(&a, 2), part(&b, 4)),
        // Another size than the one that was signed.
        format!("/img/{}/thumbnail/{}", part(&a, 2), part(&a, 4)),
        format!("/img/{}/huge/{}", part(&a, 2), part(&a, 4)),
        // A signature that is nearly right, none, and one made with another key.
        format!("/img/{flipped}/image/{}", part(&a, 4)),
        format!("/img/AAAAAAAAAAAAAAAAAAAAAA/image/{}", part(&a, 4)),
        format!("/img/x/image/{}", part(&a, 4)),
        other_key
            .address(Size::Image, &format!("{at}/a.png"))
            .unwrap(),
        // Not an address at all.
        format!("/img/{}/image/not%20base64", part(&a, 2)),
        format!("/img/{}/image/", part(&a, 2)),
    ] {
        let (status, _, body) = get_path(state.clone(), &forged).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{forged}");
        assert!(body.is_empty());
    }
    assert!(site.lock().unwrap().asked.is_empty());

    // The real ones are answered, and a server without a proxy answers none.
    assert_eq!(get_path(state, &a).await.0, StatusCode::OK);
    assert_eq!(
        get_path(AppState::new(db), &a).await.0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn what_is_no_image_or_too_large_or_out_of_reach_is_refused(db: PgPool) {
    let (at, site) = site(&[
        (
            "page.png",
            StatusCode::OK,
            "image/png",
            b"<html><script>alert(1)</script></html>".to_vec(),
        ),
        (
            "drawing.svg",
            StatusCode::OK,
            "image/svg+xml",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_vec(),
        ),
        (
            "huge.png",
            StatusCode::OK,
            "image/png",
            picture(900, 900, ImageFormat::Png),
        ),
        (
            "gone.png",
            StatusCode::GONE,
            "image/png",
            picture(4, 4, ImageFormat::Png),
        ),
    ])
    .await;
    assert!(site.lock().unwrap().files["huge.png"].2.len() > 300_000);
    let images = proxy("refused", true);
    let state = AppState::new(db.clone()).with_images(images.clone());
    let address = |name: &str| {
        images
            .signer()
            .address(Size::Image, &format!("{at}/{name}"))
            .unwrap()
    };

    for (name, status) in [
        ("page.png", StatusCode::UNSUPPORTED_MEDIA_TYPE),
        ("drawing.svg", StatusCode::UNSUPPORTED_MEDIA_TYPE),
        ("huge.png", StatusCode::PAYLOAD_TOO_LARGE),
        ("gone.png", StatusCode::BAD_GATEWAY),
        ("missing.png", StatusCode::BAD_GATEWAY),
    ] {
        let (answer, _, body) = get_path(state.clone(), &address(name)).await;
        assert_eq!(answer, status, "{name}");
        // Nothing of what the source sent reaches the reader.
        assert!(body.is_empty(), "{name}");
    }
    assert_eq!(images.cache().used(), 0);

    // A source that failed is left alone for a while.
    for name in ["page.png", "gone.png"] {
        let (answer, _, _) = get_path(state.clone(), &address(name)).await;
        assert_eq!(answer, StatusCode::BAD_GATEWAY, "{name}");
        assert_eq!(asked(&site, name), 1, "{name}");
    }

    // An article can name an image inside the server's own network. The
    // address gets signed like any other, and the client refuses to go there.
    let strict = proxy("internal", false);
    let state = AppState::new(db).with_images(strict.clone());
    let before = site.lock().unwrap().asked.len();
    for inside in [
        format!("{at}/huge.png"),
        "http://localhost/a.png".to_owned(),
        "http://169.254.169.254/latest/meta-data/".to_owned(),
        "http://[::1]/a.png".to_owned(),
        "http://192.168.1.1/logo.png".to_owned(),
    ] {
        let address = strict.signer().address(Size::Image, &inside).unwrap();
        let (answer, _, body) = get_path(state.clone(), &address).await;
        assert_eq!(answer, StatusCode::BAD_GATEWAY, "{inside}");
        assert!(body.is_empty());
    }
    assert_eq!(site.lock().unwrap().asked.len(), before);
}

#[test]
fn the_proxy_keeps_a_key_of_its_own() {
    let dir = cache_dir("key");
    let client = || HttpClient::new(http_client::Config::default()).unwrap();
    let first = ImageProxy::open(Config::new(&dir), client()).unwrap();
    let address = first
        .signer()
        .address(Size::Icon, "https://example.org/i.png");
    let key = std::fs::read(dir.join("key")).unwrap();
    assert_eq!(key.len(), 32);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.join("key"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    // After a restart the addresses are the same, so what browsers kept still fits.
    let second = ImageProxy::open(Config::new(&dir), client()).unwrap();
    assert_eq!(
        second
            .signer()
            .address(Size::Icon, "https://example.org/i.png"),
        address
    );
    // The key is no image of the cache.
    assert_eq!(second.cache().used(), 0);
    second.cache().sweep();
    assert!(dir.join("key").exists());

    // A key from the operator is used and not written down.
    let own = cache_dir("own-key");
    let config = Config {
        key: Some(KEY.to_vec()),
        ..Config::new(&own)
    };
    let third = ImageProxy::open(config, client()).unwrap();
    assert_ne!(
        third
            .signer()
            .address(Size::Icon, "https://example.org/i.png"),
        address
    );
    assert!(!own.join("key").exists());
    let config = Config {
        key: Some(b"too short".to_vec()),
        ..Config::new(&own)
    };
    assert!(ImageProxy::open(config, client()).is_err());
    for dir in [dir, own] {
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[sqlx::test]
async fn a_delivered_text_has_its_images_from_the_proxy(db: PgPool) {
    let (at, _site) = site(&[(
        "a.png",
        StatusCode::OK,
        "image/png",
        picture(40, 30, ImageFormat::Png),
    )])
    .await;
    let images = proxy("delivered", true);
    let signer = images.signer();
    // What is stored: cleaned, with the images' own addresses.
    let stored = clean(
        &format!(
            "<p>One <img src=\"{at}/a.png?utm_source=x&amp;v=1\" alt=\"A &amp; &quot;B&quot;\" width=\"40\"> \
             and <a href=\"https://example.org/b.png\">a link to <img src=\"b.png\"></a>.</p>\
             <pre>  &lt;img src=\"https://example.org/in-text.png\"&gt;  </pre>"
        ),
        Base {
            article: Some("https://example.org/posts/1"),
            feed: None,
        },
        &Rules::built_in().unwrap(),
    )
    .html;
    assert!(stored.contains(&format!("src=\"{at}/a.png?v=1\"")));

    let delivered = signer.in_html(&stored);
    let first = signer
        .address(Size::Image, &format!("{at}/a.png?v=1"))
        .unwrap();
    let second = signer
        .address(Size::Image, "https://example.org/posts/b.png")
        .unwrap();
    assert_eq!(
        delivered,
        format!(
            "<p>One <img src=\"{first}\" alt=\"A &amp; &quot;B&quot;\" width=\"40\" \
             title=\"A &amp; &quot;B&quot;\" loading=\"lazy\"> and \
             <a href=\"https://example.org/b.png\" target=\"_blank\" \
             rel=\"nofollow noopener noreferrer\">a link to <img src=\"{second}\" \
             loading=\"lazy\"></a>.</p>\
             <pre>  &lt;img src=\"https://example.org/in-text.png\"&gt;  </pre>"
        )
    );
    // No image is left that a browser would fetch from anywhere but rueder.
    for tag in delivered.split('<').filter(|tag| tag.starts_with("img ")) {
        assert!(tag.contains(" src=\"/img/"), "{tag}");
    }
    // And the address in the text is one the proxy answers.
    let state = AppState::new(db).with_images(images.clone());
    assert_eq!(get_path(state, &first).await.0, StatusCode::OK);

    // An image without an address the proxy can serve does not go out.
    assert_eq!(
        signer.in_html("<p>a<img src=\"/api/logout\"><img alt=\"x\"><img src=\"data:image/png;base64,AA\">b</p>"),
        "<p>ab</p>"
    );
    assert_eq!(signer.in_html(""), "");
    assert_eq!(signer.in_html("no images here"), "no images here");
}
