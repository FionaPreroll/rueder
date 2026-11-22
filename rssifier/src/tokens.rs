//! Who may ask: app tokens, each with a name and optional limits.
//!
//! A token is `rsf_` and 24 letters and digits of base32, 120 bits, like an
//! app password of rueder. It is shown once, when it is made; the file keeps
//! only its SHA-256. The file belongs to the operator and is written by
//! `rssifier token`; the server reads it again when it changes, so a revoked
//! token ends with the next request.

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
    time::SystemTime,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

/// What a token starts with, so that it is recognized where it turns up.
pub const PREFIX: &str = "rsf_";
/// The letters of base32, in lower case.
const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
/// How many letters follow the prefix.
const LETTERS: usize = 24;

/// What one token may do. A limit that is not set does not hold.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Limits {
    /// Requests per minute, from the cache or not.
    pub per_minute: Option<u32>,
    /// Pages fetched per hour: the requests the cache could not answer, which
    /// cost the sites something.
    pub fetches_per_hour: Option<u32>,
    /// Requests under way at the same time.
    pub at_once: Option<u32>,
}

/// A token as the file keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub name: String,
    /// The SHA-256 of the token, in hex.
    pub hash: String,
    #[serde(default)]
    pub limits: Limits,
}

#[derive(Default, Serialize, Deserialize)]
struct File {
    tokens: Vec<Token>,
}

/// A new token.
pub fn new_secret() -> anyhow::Result<String> {
    let mut bytes = [0u8; LETTERS];
    getrandom::fill(&mut bytes).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
    // 256 is a multiple of 32, so every letter is as likely as any other.
    let letters: String = bytes
        .iter()
        .map(|byte| char::from(ALPHABET[usize::from(byte & 31)]))
        .collect();
    Ok(format!("{PREFIX}{letters}"))
}

/// The SHA-256 of a token, in hex, as the file keeps it.
pub fn hash(secret: &str) -> String {
    Sha256::digest(secret.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The tokens in the file; none if there is no file yet.
pub fn load(path: &Path) -> anyhow::Result<Vec<Token>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("reading {}", path.display())),
    };
    let file: File =
        serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))?;
    Ok(file.tokens)
}

/// Writes the tokens to the file, readable only by its owner, in one step: a
/// server reading it meanwhile sees the old file or the new one.
pub fn save(path: &Path, tokens: &[Token]) -> anyhow::Result<()> {
    let file = File {
        tokens: tokens.to_vec(),
    };
    let text = serde_json::to_string_pretty(&file)?;
    let mut next = path.as_os_str().to_owned();
    next.push(".new");
    let next = PathBuf::from(next);
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    io::Write::write_all(&mut options.open(&next)?, text.as_bytes())
        .with_context(|| format!("writing {}", next.display()))?;
    fs::rename(&next, path).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// The tokens the server knows, from the file, read again when it changed.
pub struct Keeper {
    path: PathBuf,
    read: Mutex<(Option<SystemTime>, Arc<Vec<Token>>)>,
}

impl Keeper {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let tokens = load(&path)?;
        let modified = fs::metadata(&path).and_then(|meta| meta.modified()).ok();
        Ok(Self {
            path,
            read: Mutex::new((modified, Arc::new(tokens))),
        })
    }

    /// The token with this secret, if there is one. A file that cannot be read
    /// leaves the tokens as they were, and says so in the log.
    pub fn find(&self, secret: &str) -> Option<Token> {
        let tokens = self.current();
        let hash = hash(secret);
        tokens.iter().find(|token| token.hash == hash).cloned()
    }

    fn current(&self) -> Arc<Vec<Token>> {
        let modified = fs::metadata(&self.path)
            .and_then(|meta| meta.modified())
            .ok();
        let mut read = self.read.lock().unwrap_or_else(PoisonError::into_inner);
        if modified != read.0 {
            match load(&self.path) {
                Ok(tokens) => *read = (modified, Arc::new(tokens)),
                Err(err) => tracing::warn!("keeping the tokens as they were: {err:#}"),
            }
        }
        read.1.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_token_has_the_prefix_and_120_bits_of_base32() {
        let secret = new_secret().unwrap();
        let letters = secret.strip_prefix(PREFIX).unwrap();
        assert_eq!(letters.len(), LETTERS);
        assert!(letters.bytes().all(|b| ALPHABET.contains(&b)));
        assert_ne!(secret, new_secret().unwrap());
        assert_eq!(hash(&secret).len(), 64);
    }
}
