//! The image proxy: every image a reader sees comes from rueder, never from the
//! site that published it, so that site learns nothing about who reads what.
//!
//! - **Addresses are signed.** `/img/<signature>/<size>/<address>` carries an
//!   HMAC over size and address, made with a key only the server has. The
//!   server puts these addresses into what it delivers ([`Signer::address`],
//!   [`Signer::in_html`]); nobody else can make one, so the proxy fetches only
//!   what stood in an article.
//! - **Only images pass.** PNG, JPEG, GIF and WebP, told by their first bytes.
//!   Anything else is refused, SVG included, which is a program more than a
//!   picture.
//! - **Fixed sizes.** An image larger than its size allows is made smaller;
//!   the limits for bytes and pixels are checked before anything is decoded.
//! - **A cache on disk**, limited in size, so a source is asked once.
//!
//! Fetching goes through the hardened HTTP client like everything else.

use std::{
    collections::HashMap,
    fmt, fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Context, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use image::{
    DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, codecs::jpeg::JpegEncoder,
};
use sha2::{Digest, Sha256};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use url::Url;

use crate::{
    cleaning,
    http_client::{self, HttpClient, Request},
    monitor::{self, Window},
};

/// The most pixels an image may have. Decoding takes four bytes for each.
pub const MAX_PIXELS: u64 = 30_000_000;
/// The longest address the proxy takes.
const MAX_ADDRESS: usize = 2000;
/// How many sources are asked at a time, and how many images are decoded.
const FETCHES_AT_ONCE: usize = 8;
const DECODES_AT_ONCE: usize = 2;
/// How many of those go to one host at a time: a source that answers slowly,
/// or a feed full of images from one place, leaves room for the others.
const FETCHES_PER_HOST: usize = 4;
/// How long a source that failed is left alone.
const FAILED_FOR: Duration = Duration::from_secs(5 * 60);
/// How many failed sources are remembered.
const FAILED_AT_MOST: usize = 10_000;
/// When the cache is over its limit, it is emptied down to this share of it.
const SWEEP_TO: f64 = 0.8;

const ACCEPT: &str = "image/png, image/jpeg, image/gif, image/webp";

/// What an image is for, which says how large it may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// A feed's icon.
    Icon,
    /// The picture next to an article in a list.
    Thumbnail,
    /// An image in an article.
    Image,
}

impl Size {
    const ALL: [Self; 3] = [Self::Icon, Self::Thumbnail, Self::Image];

    fn name(self) -> &'static str {
        match self {
            Self::Icon => "icon",
            Self::Thumbnail => "thumbnail",
            Self::Image => "image",
        }
    }

    fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|size| size.name() == name)
    }

    /// The width and height an image of this size fits into.
    pub fn fits(self) -> (u32, u32) {
        match self {
            Self::Icon => (256, 256),
            Self::Thumbnail => (800, 800),
            Self::Image => (2000, 2000),
        }
    }
}

/// Makes and checks the addresses of the proxy.
#[derive(Clone)]
pub struct Signer {
    key: Vec<u8>,
}

impl Signer {
    /// `key` is the secret the signatures are made with, 32 bytes or more.
    pub fn new(key: Vec<u8>) -> anyhow::Result<Self> {
        if key.len() < 32 {
            bail!("the key for image addresses needs at least 32 bytes");
        }
        Ok(Self { key })
    }

    fn signature(&self, size: Size, url: &str) -> Option<Hmac<Sha256>> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).ok()?;
        mac.update(size.name().as_bytes());
        mac.update(b"\n");
        mac.update(url.as_bytes());
        Some(mac)
    }

    /// The address under which the proxy serves the image at `url`, or `None`
    /// if that is not a web address.
    pub fn address(&self, size: Size, url: &str) -> Option<String> {
        if url.len() > MAX_ADDRESS || !is_web(url) {
            return None;
        }
        let signature = self.signature(size, url)?.finalize().into_bytes();
        let signature = URL_SAFE_NO_PAD.encode(signature.get(..16)?);
        let address = URL_SAFE_NO_PAD.encode(url);
        Some(format!("/img/{signature}/{}/{address}", size.name()))
    }

    /// The size and the source an address of the proxy stands for, if the
    /// server itself made the address.
    fn open(&self, signature: &str, size: &str, address: &str) -> Option<(Size, String)> {
        let size = Size::named(size)?;
        let url = String::from_utf8(URL_SAFE_NO_PAD.decode(address).ok()?).ok()?;
        let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
        if signature.len() != 16 || url.len() > MAX_ADDRESS || !is_web(&url) {
            return None;
        }
        // Compares without telling by its timing how much was right.
        self.signature(size, &url)?
            .verify_truncated_left(&signature)
            .ok()?;
        Some((size, url))
    }

    /// A cleaned text as it is delivered: every image points to the proxy. The
    /// stored text keeps the images' own addresses, so a new key or another
    /// layout of the addresses needs nothing cleaned again.
    pub fn in_html(&self, html: &str) -> String {
        cleaning::with_images(html, |source| self.address(Size::Image, source))
    }

    /// The signature of an address for an app: the number of its app password
    /// goes into it, so that the address holds only as long as that does.
    fn app_signature(&self, app: i32, size: Size, url: &str) -> Option<Hmac<Sha256>> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).ok()?;
        mac.update(b"app\n");
        mac.update(app.to_string().as_bytes());
        mac.update(b"\n");
        mac.update(size.name().as_bytes());
        mac.update(b"\n");
        mac.update(url.as_bytes());
        Some(mac)
    }

    /// The address under which the proxy serves the image at `url` to an app
    /// of another maker, which loads it without a session: `/greader/img/`,
    /// then the number of the app password, the signature, the size and the
    /// source. `None` if `url` is no web address.
    pub fn app_address(&self, app: i32, size: Size, url: &str) -> Option<String> {
        if url.len() > MAX_ADDRESS || !is_web(url) {
            return None;
        }
        let signature = self.app_signature(app, size, url)?.finalize().into_bytes();
        let signature = URL_SAFE_NO_PAD.encode(signature.get(..16)?);
        let address = URL_SAFE_NO_PAD.encode(url);
        Some(format!("/greader/img/{app}/{signature}/{}/{address}", size.name()))
    }

    /// What an app's address of the proxy stands for, if the server made it
    /// for this app password.
    fn open_for_app(
        &self,
        app: i32,
        signature: &str,
        size: &str,
        address: &str,
    ) -> Option<(Size, String)> {
        let size = Size::named(size)?;
        let url = String::from_utf8(URL_SAFE_NO_PAD.decode(address).ok()?).ok()?;
        let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
        if signature.len() != 16 || url.len() > MAX_ADDRESS || !is_web(&url) {
            return None;
        }
        self.app_signature(app, size, &url)?
            .verify_truncated_left(&signature)
            .ok()?;
        Some((size, url))
    }

    /// A cleaned text as an app gets it: every image points to the proxy, at
    /// the full address of this server (`origin`), as apps show texts in a
    /// page of their own.
    pub fn in_html_for_app(&self, app: i32, origin: &str, html: &str) -> String {
        cleaning::with_images(html, |source| {
            let address = self.app_address(app, Size::Image, source)?;
            Some(format!("{origin}{address}"))
        })
    }
}

fn is_web(url: &str) -> bool {
    Url::parse(url)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

/// The kinds of image the proxy lets through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
    Gif,
    WebP,
}

impl Format {
    const ALL: [Self; 4] = [Self::Png, Self::Jpeg, Self::Gif, Self::WebP];

    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::WebP => "image/webp",
        }
    }

    /// How the format is written down in the cache.
    fn code(self) -> u8 {
        match self {
            Self::Png => 1,
            Self::Jpeg => 2,
            Self::Gif => 3,
            Self::WebP => 4,
        }
    }

    fn coded(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|format| format.code() == code)
    }
}

/// An image as the proxy serves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub format: Format,
    pub bytes: Vec<u8>,
}

/// Why the proxy does not serve an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// Not an address the server made.
    NotFound,
    /// The source could not be fetched; the reason is for the log.
    Source(String),
    /// What the source sent is none of the formats the proxy lets through.
    NotAnImage,
    /// More bytes or more pixels than the proxy takes.
    TooLarge,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "not an address of the image proxy"),
            Self::Source(why) => write!(f, "the source could not be fetched: {why}"),
            Self::NotAnImage => write!(f, "not an image the proxy lets through"),
            Self::TooLarge => write!(f, "the image is too large"),
        }
    }
}

/// Checks that `bytes` are an image and makes it fit `size`.
///
/// An image that fits already passes as it came, animation included. A larger
/// one is decoded, made smaller and written anew, as PNG if it has
/// transparency and as JPEG otherwise; of an animation the first picture is
/// left. Dimensions are read from the head of the file and checked before
/// anything is decoded, so a small file that unpacks into gigabytes is refused.
pub fn fit(bytes: Vec<u8>, size: Size) -> Result<Picture, Refused> {
    // Told by the first bytes, never by what the source calls it.
    let format = match image::guess_format(&bytes) {
        Ok(ImageFormat::Png) => Format::Png,
        Ok(ImageFormat::Jpeg) => Format::Jpeg,
        Ok(ImageFormat::Gif) => Format::Gif,
        Ok(ImageFormat::WebP) => Format::WebP,
        _ => return Err(Refused::NotAnImage),
    };
    let (max_width, max_height) = size.fits();
    let resized = {
        let mut reader = ImageReader::new(Cursor::new(&bytes));
        reader.set_format(match format {
            Format::Png => ImageFormat::Png,
            Format::Jpeg => ImageFormat::Jpeg,
            Format::Gif => ImageFormat::Gif,
            Format::WebP => ImageFormat::WebP,
        });
        let mut limits = Limits::default();
        limits.max_alloc = Some(MAX_PIXELS * 4 + 64 * 1024 * 1024);
        reader.limits(limits);
        let mut decoder = reader.into_decoder().map_err(|_| Refused::NotAnImage)?;
        let (width, height) = decoder.dimensions();
        if width == 0 || height == 0 {
            return Err(Refused::NotAnImage);
        }
        if u64::from(width) * u64::from(height) > MAX_PIXELS {
            return Err(Refused::TooLarge);
        }
        if width <= max_width && height <= max_height {
            None
        } else {
            // Cameras write photos lying on their side and note which way is up.
            let orientation = decoder.orientation().ok();
            let mut image = DynamicImage::from_decoder(decoder).map_err(|_| Refused::NotAnImage)?;
            if let Some(orientation) = orientation {
                image.apply_orientation(orientation);
            }
            Some(image.thumbnail(max_width, max_height))
        }
    };
    let Some(image) = resized else {
        return Ok(Picture { format, bytes });
    };
    let mut out = Vec::new();
    // Formats that can be seen through are decoded with that in them even if
    // every pixel is solid; only a picture that makes use of it needs a PNG.
    let see_through =
        image.color().has_alpha() && image.to_rgba8().pixels().any(|pixel| pixel.0[3] != u8::MAX);
    let format = if see_through {
        image
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .map_err(|_| Refused::NotAnImage)?;
        Format::Png
    } else {
        JpegEncoder::new_with_quality(&mut out, 85)
            .encode_image(&image.to_rgb8())
            .map_err(|_| Refused::NotAnImage)?;
        Format::Jpeg
    };
    Ok(Picture { format, bytes: out })
}

/// Images on disk, so that a source is asked once. One file per image and
/// size, named by a checksum of both. The cache holds nothing that cannot be
/// fetched again; when it is over its limit, the files unused longest go.
pub struct Cache {
    dir: PathBuf,
    max_bytes: u64,
    used: AtomicU64,
    sweeping: AtomicBool,
}

impl Cache {
    /// Opens the cache in `dir`, which is made if it is not there.
    pub fn open(dir: &Path, max_bytes: u64) -> anyhow::Result<Self> {
        fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
        let cache = Self {
            dir: dir.to_owned(),
            max_bytes,
            used: AtomicU64::new(0),
            sweeping: AtomicBool::new(false),
        };
        let used = cache.files().iter().map(|file| file.bytes).sum();
        cache.used.store(used, Ordering::Relaxed);
        Ok(cache)
    }

    fn path(&self, size: Size, url: &str) -> PathBuf {
        let digest = Sha256::new()
            .chain_update(size.name())
            .chain_update(b"\n")
            .chain_update(url)
            .finalize();
        let name: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        let (shard, _) = name.split_at(2);
        self.dir.join(shard).join(name)
    }

    /// The image, if the cache has it. Having been asked for keeps it longer.
    pub fn get(&self, size: Size, url: &str) -> Option<Picture> {
        let path = self.path(size, url);
        let mut bytes = fs::read(&path).ok()?;
        let format = Format::coded(*bytes.first()?)?;
        bytes.remove(0);
        if let Ok(file) = fs::File::options().write(true).open(&path) {
            let _ = file.set_modified(SystemTime::now());
        }
        Some(Picture { format, bytes })
    }

    /// Keeps an image. Failing to is no reason to fail the request it came from.
    pub fn put(&self, size: Size, url: &str, picture: &Picture) {
        let path = self.path(size, url);
        let written = (|| {
            let dir = path.parent()?;
            fs::create_dir_all(dir).ok()?;
            // Written beside its place and moved there, so nobody reads half a file.
            let mut name = [0_u8; 8];
            getrandom::fill(&mut name).ok()?;
            let name: String = name.iter().map(|byte| format!("{byte:02x}")).collect();
            let partial = dir.join(format!("{name}.partial"));
            let mut file = fs::File::create(&partial).ok()?;
            let stored = file
                .write_all(&[picture.format.code()])
                .and_then(|()| file.write_all(&picture.bytes))
                .and_then(|()| fs::rename(&partial, &path));
            if stored.is_err() {
                let _ = fs::remove_file(&partial);
            }
            stored.ok()
        })();
        if written.is_some() {
            let size = picture.bytes.len() as u64 + 1;
            if self.used.fetch_add(size, Ordering::Relaxed) + size > self.max_bytes {
                self.sweep();
            }
        }
    }

    /// Takes an image out of the cache in every size; how many files went.
    pub fn forget(&self, url: &str) -> usize {
        Size::ALL
            .into_iter()
            .filter(|size| fs::remove_file(self.path(*size, url)).is_ok())
            .count()
    }

    /// How many bytes the cache holds, as far as it has counted.
    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    /// How many bytes it may hold.
    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    /// Removes the files unused longest until the cache is well under its
    /// limit, and files a write left behind unfinished.
    pub fn sweep(&self) {
        if self.sweeping.swap(true, Ordering::Acquire) {
            return;
        }
        let mut files = self.files();
        files.sort_by_key(|file| file.used_at);
        let mut used: u64 = files.iter().map(|file| file.bytes).sum();
        let target = (self.max_bytes as f64 * SWEEP_TO) as u64;
        let stale = SystemTime::now() - Duration::from_secs(60 * 60);
        for file in files {
            let partial = file.path.extension().is_some_and(|ext| ext == "partial");
            let goes = if partial {
                file.used_at < stale
            } else {
                used > target
            };
            if goes && fs::remove_file(&file.path).is_ok() {
                used -= file.bytes;
            }
        }
        self.used.store(used, Ordering::Relaxed);
        self.sweeping.store(false, Ordering::Release);
    }

    /// Every file of the cache.
    fn files(&self) -> Vec<Cached> {
        let mut files = Vec::new();
        let shards = fs::read_dir(&self.dir).into_iter().flatten().flatten();
        for shard in shards.filter(|shard| shard.path().is_dir()) {
            for file in fs::read_dir(shard.path()).into_iter().flatten().flatten() {
                if let Ok(meta) = file.metadata() {
                    files.push(Cached {
                        path: file.path(),
                        bytes: meta.len(),
                        used_at: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    });
                }
            }
        }
        files
    }
}

struct Cached {
    path: PathBuf,
    bytes: u64,
    used_at: SystemTime,
}

/// How the proxy is set up.
#[derive(Debug, Clone)]
pub struct Config {
    /// Where the cache lies. The only place the server writes files to.
    pub cache_dir: PathBuf,
    /// How large the cache may get, in bytes.
    pub cache_bytes: u64,
    /// The secret for the signatures. Without one, the proxy makes its own on
    /// first start and keeps it in the file `key` of the cache directory.
    pub key: Option<Vec<u8>>,
    /// The largest image file the proxy fetches, in bytes.
    pub max_source: usize,
}

impl Config {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            cache_bytes: 1024 * 1024 * 1024,
            key: None,
            max_source: 20 * 1024 * 1024,
        }
    }
}

/// The image proxy.
pub struct ImageProxy {
    signer: Signer,
    client: HttpClient,
    cache: Arc<Cache>,
    max_source: usize,
    fetches: Semaphore,
    hosts: Hosts,
    decodes: Arc<Semaphore>,
    /// Sources that failed, and until when they are left alone.
    failed: Mutex<HashMap<String, Instant>>,
    /// What it served in the last hour, for the status page.
    served: Mutex<Window<monitor::Images>>,
}

impl ImageProxy {
    pub fn open(config: Config, client: HttpClient) -> anyhow::Result<Self> {
        let cache = Cache::open(&config.cache_dir, config.cache_bytes)?;
        let key = match config.key {
            Some(key) => key,
            None => own_key(&config.cache_dir.join("key"))?,
        };
        Ok(Self {
            signer: Signer::new(key)?,
            client,
            cache: Arc::new(cache),
            max_source: config.max_source,
            fetches: Semaphore::new(FETCHES_AT_ONCE),
            hosts: Hosts::default(),
            decodes: Arc::new(Semaphore::new(DECODES_AT_ONCE)),
            failed: Mutex::default(),
            served: Mutex::default(),
        })
    }

    /// Makes the addresses this proxy serves.
    pub fn signer(&self) -> &Signer {
        &self.signer
    }

    pub fn cache(&self) -> &Cache {
        &self.cache
    }

    /// What the proxy served in the last hour.
    pub fn served(&self, now: Instant) -> monitor::Images {
        self.served
            .lock()
            .map(|window| window.last_hour(now))
            .unwrap_or_default()
    }

    /// How many sources are left alone for now because they failed.
    pub fn failing_sources(&self) -> usize {
        let now = Instant::now();
        self.failed.lock().map_or(0, |failed| {
            failed.values().filter(|until| **until > now).count()
        })
    }

    /// The image behind an address of the proxy: from the cache, or fetched
    /// from its source, checked, made to fit and kept.
    pub async fn get(
        &self,
        signature: &str,
        size: &str,
        address: &str,
    ) -> Result<Picture, Refused> {
        self.deliver(self.signer.open(signature, size, address))
            .await
    }

    /// The image behind an address the proxy made for an app (see
    /// [`Signer::app_address`]). That its app password still counts is the
    /// caller's to check.
    pub async fn get_for_app(
        &self,
        app: i32,
        signature: &str,
        size: &str,
        address: &str,
    ) -> Result<Picture, Refused> {
        let opened = self.signer.open_for_app(app, signature, size, address);
        self.deliver(opened).await
    }

    /// The image an address stands for, if the server made the address.
    async fn deliver(&self, opened: Option<(Size, String)>) -> Result<Picture, Refused> {
        let started = Instant::now();
        let found = self.find(opened).await;
        let cached = matches!(found, Ok((_, true)));
        let got = found.map(|(picture, _)| picture);
        let now = Instant::now();
        if let Ok(mut window) = self.served.lock() {
            let served = window.at(now);
            match &got {
                Ok(_) if cached => served.cached += 1,
                Ok(_) => served.fetched += 1,
                Err(Refused::NotFound) => served.not_found += 1,
                Err(Refused::Source(_)) => served.source_failed += 1,
                Err(Refused::NotAnImage) => served.not_image += 1,
                Err(Refused::TooLarge) => served.too_large += 1,
            }
            if !cached && !matches!(got, Err(Refused::NotFound)) {
                served.timing.note(now - started);
            }
        }
        got
    }

    /// The image, and whether it came from the cache.
    async fn find(&self, opened: Option<(Size, String)>) -> Result<(Picture, bool), Refused> {
        let (size, url) = opened.ok_or(Refused::NotFound)?;
        let cached = {
            let (cache, url) = (self.cache.clone(), url.clone());
            tokio::task::spawn_blocking(move || cache.get(size, &url)).await
        };
        if let Ok(Some(picture)) = cached {
            return Ok((picture, true));
        }
        if self.has_failed(&url) {
            return Err(Refused::Source("it failed a moment ago".into()));
        }
        let fetched = self.fetch(size, &url).await;
        if fetched.is_err() {
            self.note_failed(url);
        }
        fetched.map(|picture| (picture, false))
    }

    async fn fetch(&self, size: Size, url: &str) -> Result<Picture, Refused> {
        let bytes = {
            // The host's turn first: who waits for a busy host holds up no
            // other.
            let host = Url::parse(url)
                .ok()
                .and_then(|url| url.host_str().map(str::to_owned))
                .unwrap_or_default();
            let _turn = self.hosts.enter(host).await;
            let _permit = self.fetches.acquire().await;
            // No cookies, no referrer, and always the same user agent: the
            // source cannot tell one reader from another.
            let request = Request {
                accept: Some(ACCEPT),
                max_body: Some(self.max_source),
                ..Request::new(url)
            };
            match self.client.get(request).await {
                Ok(response) if response.status.is_success() => response.body,
                Ok(response) => {
                    return Err(Refused::Source(format!("it answers {}", response.status)));
                }
                Err(http_client::Error::TooLarge(_)) => return Err(Refused::TooLarge),
                Err(err) => return Err(Refused::Source(err.to_string())),
            }
        };
        // Decoding takes time and memory: off the threads that serve requests,
        // and only a few at once.
        let permit = self.decodes.clone().acquire_owned().await;
        let (cache, url) = (self.cache.clone(), url.to_owned());
        let fitting = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let picture = fit(bytes, size)?;
            cache.put(size, &url, &picture);
            Ok(picture)
        });
        match fitting.await {
            Ok(result) => result,
            Err(_) => Err(Refused::NotAnImage),
        }
    }

    fn has_failed(&self, url: &str) -> bool {
        let Ok(failed) = self.failed.lock() else {
            return false;
        };
        failed.get(url).is_some_and(|until| *until > Instant::now())
    }

    fn note_failed(&self, url: String) {
        let Ok(mut failed) = self.failed.lock() else {
            return;
        };
        let now = Instant::now();
        if failed.len() >= FAILED_AT_MOST {
            failed.retain(|_, until| *until > now);
        }
        if failed.len() >= FAILED_AT_MOST {
            failed.clear();
        }
        failed.insert(url, now + FAILED_FOR);
    }
}

/// How many fetches go to each host right now. A host is forgotten as soon as
/// nobody fetches from it or waits for it.
#[derive(Default)]
struct Hosts {
    busy: Mutex<HashMap<String, Arc<Semaphore>>>,
}

impl Hosts {
    /// Waits until a fetch from `host` may start.
    async fn enter(&self, host: String) -> HostTurn<'_> {
        let semaphore = match self.busy.lock() {
            Ok(mut busy) => busy
                .entry(host.clone())
                .or_insert_with(|| Arc::new(Semaphore::new(FETCHES_PER_HOST)))
                .clone(),
            // Only a panic while the lock was held poisons it; then every
            // fetch gets a turn of its own rather than none.
            Err(_) => Arc::new(Semaphore::new(FETCHES_PER_HOST)),
        };
        let permit = semaphore.clone().acquire_owned().await.ok();
        HostTurn {
            hosts: self,
            host,
            semaphore,
            permit,
        }
    }

    #[cfg(test)]
    fn known(&self) -> usize {
        self.busy.lock().map_or(0, |busy| busy.len())
    }
}

struct HostTurn<'a> {
    hosts: &'a Hosts,
    host: String,
    semaphore: Arc<Semaphore>,
    permit: Option<OwnedSemaphorePermit>,
}

impl Drop for HostTurn<'_> {
    fn drop(&mut self) {
        drop(self.permit.take());
        let Ok(mut busy) = self.hosts.busy.lock() else {
            return;
        };
        // The map and this turn hold the only references: nobody else fetches
        // from the host or waits for it. Others only take one under the lock.
        if Arc::strong_count(&self.semaphore) == 2
            && busy
                .get(&self.host)
                .is_some_and(|known| Arc::ptr_eq(known, &self.semaphore))
        {
            busy.remove(&self.host);
        }
    }
}

/// The key the proxy keeps for itself: read from `path`, or made and written
/// there on first start. It stays the same across restarts, so addresses that
/// browsers remember keep working.
fn own_key(path: &Path) -> anyhow::Result<Vec<u8>> {
    if let Ok(key) = fs::read(path)
        && key.len() >= 32
    {
        return Ok(key);
    }
    let mut key = [0_u8; 32];
    getrandom::fill(&mut key).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
    let mut options = fs::File::options();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options
        .open(path)
        .and_then(|mut file| file.write_all(&key))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(key.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn one_host_gets_only_some_of_the_fetches() {
        let hosts = Hosts::default();
        let mut turns = Vec::new();
        for _ in 0..FETCHES_PER_HOST {
            turns.push(hosts.enter("a.example".into()).await);
        }
        let waiting =
            tokio::time::timeout(Duration::from_millis(50), hosts.enter("a.example".into())).await;
        assert!(waiting.is_err(), "one more fetch from a busy host waits");

        let other =
            tokio::time::timeout(Duration::from_millis(50), hosts.enter("b.example".into())).await;
        assert!(other.is_ok(), "another host is not held up");
        drop(other);
        assert_eq!(hosts.known(), 1);

        turns.pop();
        let next =
            tokio::time::timeout(Duration::from_millis(50), hosts.enter("a.example".into())).await;
        assert!(next.is_ok(), "a turn that ends lets the next one in");
        drop(next);
        drop(turns);
        assert_eq!(hosts.known(), 0, "a host nobody fetches from is forgotten");
    }
}
