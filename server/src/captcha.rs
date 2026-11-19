//! The captcha of open registration: a proof of work after ALTCHA's scheme.
//!
//! The server hides a random number up to [`MAX_NUMBER`] in the SHA-256 of a
//! random salt and that number, and signs the hash and its expiry with a key
//! it makes at start. The page counts up from 0 until the hash matches and
//! sends the number with the registration. One account takes the browser a
//! moment; many take a lot of computing. It is no test of being human:
//! whatever runs the page solves it too.
//!
//! Nothing is stored in the database. A solved challenge counts once: the
//! server keeps the used ones in memory until they expire, and a restart
//! makes a new key, so that no challenge from before it is good.

use std::{
    collections::BTreeSet,
    sync::{Mutex, PoisonError},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;
use utoipa::ToSchema;

/// The highest number a challenge hides. A phone counts that far in about two
/// seconds, so it waits one on average (see "Messung" in
/// `docs/implementation.md`).
pub const MAX_NUMBER: u32 = 100_000;
/// How long a challenge is good for, in seconds. The page asks for one when it
/// sends the registration.
pub const VALID_SECONDS: i64 = 300;
/// The most solved challenges kept at a time. Past that, none counts until
/// some expire, so that nobody fills the server's memory with them.
const MAX_USED: usize = 100_000;
/// The bytes of a salt, which goes out in hex.
const SALT_BYTES: usize = 12;

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<Challenge>().add::<Solution>();
}

/// A challenge for the page to solve before it registers, in
/// `GET /api/register`.
#[derive(Debug, Clone, Serialize, TS, ToSchema)]
#[ts(rename = "CaptchaChallenge")]
#[schema(as = CaptchaChallenge)]
pub struct Challenge {
    /// Random, in hex.
    pub salt: String,
    /// The SHA-256 of the salt followed by the hidden number in decimal, in hex.
    pub challenge: String,
    /// The highest number it can be.
    pub max: u32,
    /// Until when it is good, in seconds since 1970.
    pub expires: i64,
    /// The server's signature of the challenge and its expiry.
    pub signature: String,
}

/// A solved challenge, which `POST /api/register` reads: the challenge as it
/// came, and the number that was hidden in it.
#[derive(Debug, Clone, Deserialize, TS, ToSchema)]
#[ts(rename = "CaptchaSolution")]
#[schema(as = CaptchaSolution)]
pub struct Solution {
    pub salt: String,
    pub challenge: String,
    pub expires: i64,
    pub signature: String,
    pub number: u32,
}

/// The key challenges are signed with, and the challenges solved lately.
pub struct Captcha {
    /// As long as a block of SHA-256, the most HMAC uses of a key.
    key: [u8; 64],
    /// The expiry and the challenge of each solution that counted, in the
    /// order they expire, so that those which have go from the front.
    used: Mutex<BTreeSet<(i64, String)>>,
}

impl Default for Captcha {
    fn default() -> Self {
        let mut key = [0; 64];
        getrandom::fill(&mut key).expect("random numbers");
        Self {
            key,
            used: Mutex::default(),
        }
    }
}

impl Captcha {
    /// A new challenge, good until `VALID_SECONDS` after `now` (seconds since 1970).
    pub fn challenge(&self, now: i64) -> anyhow::Result<Challenge> {
        let mut salt = [0; SALT_BYTES];
        let mut number = [0; 4];
        getrandom::fill(&mut salt).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
        getrandom::fill(&mut number).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
        let salt = hex(&salt);
        let number = u32::from_le_bytes(number) % (MAX_NUMBER + 1);
        let challenge = hash(&salt, number);
        let expires = now + VALID_SECONDS;
        let signature =
            URL_SAFE_NO_PAD.encode(self.mac(&challenge, expires).finalize().into_bytes());
        Ok(Challenge {
            salt,
            challenge,
            max: MAX_NUMBER,
            expires,
            signature,
        })
    }

    /// Whether the solution is right, of a challenge this server made, not
    /// expired and not used before. A right one counts once, whatever comes
    /// of the registration.
    pub fn check(&self, solution: &Solution, now: i64) -> bool {
        let fits = solution.salt.len() == 2 * SALT_BYTES
            && solution.challenge.len() == 64
            && solution.signature.len() <= 64
            && solution.number <= MAX_NUMBER
            && solution.expires > now
            && solution.expires <= now + VALID_SECONDS;
        if !fits {
            return false;
        }
        let Ok(signature) = URL_SAFE_NO_PAD.decode(&solution.signature) else {
            return false;
        };
        let signed = self
            .mac(&solution.challenge, solution.expires)
            .verify_slice(&signature)
            .is_ok();
        if !signed || hash(&solution.salt, solution.number) != solution.challenge {
            return false;
        }
        let mut used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
        while used.first().is_some_and(|(expires, _)| *expires <= now) {
            used.pop_first();
        }
        if used.len() >= MAX_USED {
            return false;
        }
        used.insert((solution.expires, solution.challenge.clone()))
    }

    fn mac(&self, challenge: &str, expires: i64) -> Hmac<Sha256> {
        let mut mac = Hmac::<Sha256>::new(&self.key.into());
        mac.update(format!("{challenge}.{expires}").as_bytes());
        mac
    }
}

/// The SHA-256 of the salt followed by the number in decimal, in hex.
fn hash(salt: &str, number: u32) -> String {
    hex(&Sha256::digest(format!("{salt}{number}")))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    /// Counts up to the hidden number, as the page does.
    fn solve(challenge: &Challenge) -> Solution {
        let number = (0..=challenge.max)
            .find(|&number| hash(&challenge.salt, number) == challenge.challenge)
            .unwrap();
        Solution {
            salt: challenge.salt.clone(),
            challenge: challenge.challenge.clone(),
            expires: challenge.expires,
            signature: challenge.signature.clone(),
            number,
        }
    }

    #[test]
    fn a_solution_counts_once() {
        let captcha = Captcha::default();
        let solution = solve(&captcha.challenge(NOW).unwrap());
        assert!(captcha.check(&solution, NOW + 1));
        assert!(!captcha.check(&solution, NOW + 2));
    }

    #[test]
    fn refuses_what_is_wrong() {
        let captcha = Captcha::default();
        let right = solve(&captcha.challenge(NOW).unwrap());
        let wrong = [
            Solution {
                number: (right.number + 1) % (MAX_NUMBER + 1),
                ..right.clone()
            },
            Solution {
                number: MAX_NUMBER + 1,
                ..right.clone()
            },
            Solution {
                expires: right.expires + 1,
                ..right.clone()
            },
            Solution {
                signature: "AAAA".into(),
                ..right.clone()
            },
            Solution {
                signature: "not base64!".into(),
                ..right.clone()
            },
            Solution {
                signature: "A".repeat(10_000),
                ..right.clone()
            },
            Solution {
                salt: format!("{}0", right.salt),
                ..right.clone()
            },
            Solution {
                challenge: "ü".repeat(32),
                ..right.clone()
            },
        ];
        for solution in &wrong {
            assert!(!captcha.check(solution, NOW + 1), "{solution:?}");
        }
        // Expired, and from a server with another key.
        assert!(!captcha.check(&right, right.expires));
        assert!(!Captcha::default().check(&right, NOW + 1));
        // None of that used it up.
        assert!(captcha.check(&right, NOW + 1));
    }

    #[test]
    fn keeps_no_more_than_it_may() {
        let captcha = Captcha::default();
        captcha
            .used
            .lock()
            .unwrap()
            .extend((0..MAX_USED).map(|n| (NOW + VALID_SECONDS, n.to_string())));
        let solution = solve(&captcha.challenge(NOW).unwrap());
        assert!(!captcha.check(&solution, NOW + 1));
        // Once the others expire, there is room again.
        let later = solve(&captcha.challenge(NOW + VALID_SECONDS).unwrap());
        assert!(captcha.check(&later, NOW + VALID_SECONDS + 1));
    }
}
