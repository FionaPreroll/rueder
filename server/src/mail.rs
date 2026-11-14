//! Mail to readers (work packages 5.4 and 5.5, E6): the link that confirms an
//! address, the notice to an address an account no longer has, the link that
//! sets a new password and the notice that one was set.
//!
//! Mail goes by SMTP to the one relay the operator names in `SMTP_URL`, from
//! the address in `MAIL_FROM`; the links in it lead to `PUBLIC_URL`. This is
//! the one way out of the server besides the hardened HTTP client, and a
//! deliberate one (E6): the address is the operator's, never a reader's, and
//! what a reader gives is only the address the mail goes *to*, which the relay
//! delivers. TLS is rustls with ring, as everywhere else, and checks the
//! relay's certificate against Mozilla's roots compiled in, as the HTTP client
//! does. Without TLS only a relay on
//! this machine is taken, so that neither the relay's password nor a link
//! crosses a network in the clear.
//!
//! What a mail says is data, not code: one file per mail and language in
//! `server/templates/mail/`, the first line the subject, then an empty line and
//! the text, with `{name}`-style places filled in. Mails are plain text, so
//! nothing a reader chose, such as their name, can become markup; the subject
//! holds nothing a reader chose.
//!
//! For tests there is an outbox in memory instead of the relay.

use std::{
    net::IpAddr,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use anyhow::Context;
use lettre::{
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use url::Url;
use utoipa::ToSchema;

/// How long the relay may take with one mail.
const TIMEOUT: Duration = Duration::from_secs(15);
/// The longest address there may be (RFC 5321).
pub const MAX_ADDRESS: usize = 254;

/// The languages mail is written in: those of the page.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema, sqlx::Type,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    De,
}

/// The mails there are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    /// The link that confirms an address: `{name}`, `{site}`, `{link}`,
    /// `{hours}`.
    Confirm,
    /// To the address an account had before: `{name}`, `{site}`, `{address}`
    /// (the new one, partly hidden).
    Changed,
    /// The link that sets a new password: `{name}`, `{site}`, `{link}`,
    /// `{minutes}`.
    Reset,
    /// After a new password was set with that link: `{name}`, `{site}`.
    PasswordChanged,
}

impl Template {
    pub const ALL: [Template; 4] = [
        Template::Confirm,
        Template::Changed,
        Template::Reset,
        Template::PasswordChanged,
    ];

    /// The file of the mail in this language, as it is built into the program.
    pub fn text(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::Confirm, Language::En) => include_str!("../templates/mail/en/confirm.txt"),
            (Self::Confirm, Language::De) => include_str!("../templates/mail/de/confirm.txt"),
            (Self::Changed, Language::En) => include_str!("../templates/mail/en/changed.txt"),
            (Self::Changed, Language::De) => include_str!("../templates/mail/de/changed.txt"),
            (Self::Reset, Language::En) => include_str!("../templates/mail/en/reset.txt"),
            (Self::Reset, Language::De) => include_str!("../templates/mail/de/reset.txt"),
            (Self::PasswordChanged, Language::En) => {
                include_str!("../templates/mail/en/password_changed.txt")
            }
            (Self::PasswordChanged, Language::De) => {
                include_str!("../templates/mail/de/password_changed.txt")
            }
        }
    }

    /// The places the mail fills in.
    pub fn places(self) -> &'static [&'static str] {
        match self {
            Self::Confirm => &["name", "site", "link", "hours"],
            Self::Changed => &["name", "site", "address"],
            Self::Reset => &["name", "site", "link", "minutes"],
            Self::PasswordChanged => &["name", "site"],
        }
    }
}

/// A mail as it was sent, for tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// What the outbox in memory holds.
pub type Outbox = Arc<Mutex<Vec<Sent>>>;

enum Transport {
    Smtp(AsyncSmtpTransport<Tokio1Executor>),
    Memory(Outbox),
}

/// Sends the server's mail.
pub struct Mailer {
    transport: Transport,
    from: Mailbox,
    /// Where readers reach rueder, for the links.
    site: Url,
}

impl Mailer {
    /// The mailer the environment sets up: `SMTP_URL`, `MAIL_FROM` and
    /// `PUBLIC_URL`. `None` without `SMTP_URL`; an error if one of the others
    /// is missing or none of them makes sense.
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let Ok(smtp) = std::env::var("SMTP_URL") else {
            return Ok(None);
        };
        let from = std::env::var("MAIL_FROM")
            .context("SMTP_URL needs MAIL_FROM, the address mail comes from")?;
        let site = std::env::var("PUBLIC_URL")
            .context("SMTP_URL needs PUBLIC_URL, where the links in mails lead")?;
        Self::smtp(&smtp, &from, &site).map(Some)
    }

    /// A mailer for the relay at `smtp`, as lettre reads it:
    /// `smtps://user:password@host:465`, `smtp://host:587?tls=required`, or
    /// `smtp://localhost:25` without TLS, which is taken only for this
    /// machine.
    pub fn smtp(smtp: &str, from: &str, site: &str) -> anyhow::Result<Self> {
        let url = Url::parse(smtp).map_err(|_| anyhow::anyhow!("SMTP_URL is no address"))?;
        let tls = url.query_pairs().any(|(key, _)| key == "tls");
        // `smtp:` is no scheme the URL standard knows, so its host comes as
        // text, an IP address too.
        let here = match url.host() {
            Some(url::Host::Domain(name)) => {
                name.eq_ignore_ascii_case("localhost")
                    || name
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .parse::<IpAddr>()
                        .is_ok_and(|ip| ip.is_loopback())
            }
            Some(url::Host::Ipv4(ip)) => IpAddr::V4(ip).is_loopback(),
            Some(url::Host::Ipv6(ip)) => IpAddr::V6(ip).is_loopback(),
            None => false,
        };
        match url.scheme() {
            "smtps" => {}
            "smtp"
                if url
                    .query_pairs()
                    .any(|(key, value)| key == "tls" && value == "required") => {}
            "smtp" if !tls && here => {}
            "smtp" => anyhow::bail!(
                "SMTP_URL without TLS only reaches a relay on this machine; \
                 use smtps:// or ?tls=required"
            ),
            _ => anyhow::bail!("SMTP_URL is smtp:// or smtps://"),
        }
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(smtp)
            .map_err(|err| anyhow::anyhow!("SMTP_URL: {err}"))?
            .timeout(Some(TIMEOUT))
            .build();
        Self::with(Transport::Smtp(transport), from, site)
    }

    /// A mailer that keeps what it sends, for tests.
    pub fn in_memory(site: &str) -> anyhow::Result<(Self, Outbox)> {
        let outbox = Outbox::default();
        let mailer = Self::with(
            Transport::Memory(outbox.clone()),
            "rueder <rueder@reader.example>",
            site,
        )?;
        Ok((mailer, outbox))
    }

    fn with(transport: Transport, from: &str, site: &str) -> anyhow::Result<Self> {
        let from: Mailbox = from.parse().map_err(|_| {
            anyhow::anyhow!("MAIL_FROM is no address, such as rueder <rueder@example.org>")
        })?;
        let site = Url::parse(site).map_err(|_| anyhow::anyhow!("PUBLIC_URL is no address"))?;
        Ok(Self {
            transport,
            from,
            site,
        })
    }

    /// The link of the page that takes a token, as `https://site/?confirm=…`.
    pub fn link(&self, what: &str, token: &str) -> String {
        let mut url = self.site.clone();
        url.set_path("/");
        url.query_pairs_mut().clear().append_pair(what, token);
        url.into()
    }

    /// The name of the site, as mails call it.
    pub fn site(&self) -> String {
        self.site.host_str().unwrap_or("rueder").to_owned()
    }

    /// Sends a mail to `to`, in the language given, with its places filled in.
    pub async fn send(
        &self,
        to: &str,
        language: Language,
        template: Template,
        values: &[(&str, &str)],
    ) -> anyhow::Result<()> {
        let address: Address = to.parse().context("no address mail can go to")?;
        let (subject, body) = render(template.text(language), values);
        match &self.transport {
            Transport::Memory(outbox) => {
                outbox
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(Sent {
                        to: address.to_string(),
                        subject,
                        body,
                    });
            }
            Transport::Smtp(transport) => {
                let message = Message::builder()
                    .from(self.from.clone())
                    .to(Mailbox::new(None, address))
                    .subject(subject)
                    .header(ContentType::TEXT_PLAIN)
                    .body(body)
                    .context("putting the mail together")?;
                transport
                    .send(message)
                    .await
                    .context("handing the mail to the relay")?;
            }
        }
        Ok(())
    }
}

/// The subject and the text of a mail, with its places filled in.
pub fn render(text: &str, values: &[(&str, &str)]) -> (String, String) {
    let (subject, body) = text.split_once('\n').unwrap_or((text, ""));
    let mut body = body.trim_start_matches('\n').to_owned();
    for (place, value) in values {
        body = body.replace(&format!("{{{place}}}"), value);
    }
    (subject.trim().to_owned(), body)
}

/// Whether this is an address mail can go to, as it is stored: trimmed, at
/// most [`MAX_ADDRESS`] long. `None` if it is none.
pub fn address(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() || text.len() > MAX_ADDRESS {
        return None;
    }
    let address: Address = text.parse().ok()?;
    Some(address.to_string())
}

/// An address with most of its own part hidden, as a mail to another address
/// names it: `n***@example.org`.
pub fn masked(address: &str) -> String {
    match address.split_once('@') {
        Some((user, domain)) => {
            let first: String = user.chars().take(1).collect();
            format!("{first}***@{domain}")
        }
        None => "***".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mail_has_its_places_in_every_language() {
        for template in Template::ALL {
            for language in [Language::En, Language::De] {
                let text = template.text(language);
                let (subject, body) = render(text, &[]);
                assert!(!subject.is_empty(), "{template:?} {language:?}");
                assert!(
                    !subject.contains('{'),
                    "no places in the subject: {template:?}"
                );
                for place in template.places() {
                    assert!(
                        body.contains(&format!("{{{place}}}")),
                        "{template:?} {language:?} lacks {{{place}}}"
                    );
                }
                // No other places than these.
                let filled: Vec<(&str, &str)> = template
                    .places()
                    .iter()
                    .map(|place| (*place, "x"))
                    .collect();
                let (_, body) = render(text, &filled);
                assert!(!body.contains('{'), "{template:?} {language:?}: {body}");
            }
        }
    }

    #[test]
    fn addresses_are_checked_and_masked() {
        assert_eq!(
            address("  ada@example.org "),
            Some("ada@example.org".into())
        );
        for wrong in [
            "",
            "ada",
            "ada@",
            "@example.org",
            "a b@example.org",
            "ada@exa\nmple.org",
        ] {
            assert_eq!(address(wrong), None, "{wrong:?}");
        }
        let long = format!("{}@example.org", "a".repeat(250));
        assert_eq!(address(&long), None);
        assert_eq!(masked("nina@example.org"), "n***@example.org");
    }

    #[test]
    fn a_relay_without_tls_is_only_one_on_this_machine() {
        let site = "https://reader.example";
        let from = "rueder <rueder@reader.example>";
        assert!(Mailer::smtp("smtp://127.0.0.1:2525", from, site).is_ok());
        assert!(Mailer::smtp("smtp://localhost:25", from, site).is_ok());
        assert!(Mailer::smtp("smtp://[::1]:25", from, site).is_ok());
        assert!(Mailer::smtp("smtp://relay.example:587?tls=required", from, site).is_ok());
        assert!(Mailer::smtp("smtps://user:secret@relay.example:465", from, site).is_ok());
        for wrong in [
            "smtp://relay.example:25",
            "smtp://10.0.0.1:25",
            "smtp://127.0.0.1.relay.example:25",
            "smtp://relay.example:587?tls=opportunistic",
            "http://relay.example",
            "no address",
        ] {
            assert!(Mailer::smtp(wrong, from, site).is_err(), "{wrong}");
        }
        assert!(Mailer::smtp("smtps://relay.example", "nobody", site).is_err());
    }

    #[test]
    fn links_lead_to_the_page() {
        let (mailer, _) = Mailer::in_memory("https://reader.example/some/path").unwrap();
        assert_eq!(
            mailer.link("confirm", "abc-_1"),
            "https://reader.example/?confirm=abc-_1"
        );
        assert_eq!(mailer.site(), "reader.example");
    }
}
