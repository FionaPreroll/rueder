//! The operator's legal texts: where the imprint, the privacy policy and the
//! terms of use are, and which version of the terms an account agreed to.
//!
//! The texts live on the operator's website; rueder only links to them. The
//! addresses come from `LEGAL_IMPRINT_URL`, `LEGAL_PRIVACY_URL` and
//! `LEGAL_TERMS_URL`, and anybody may ask for them (`GET /api/login/ways`), as
//! the login page shows them. With `TERMS_VERSION`, registering takes the
//! agreement to that version, and an account that agreed to another one, or to
//! none, is asked again at its next visit (`Me::terms_to_accept`, `PUT
//! /api/terms`). Without the settings rueder shows none of it.

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{AppError, AppState, sessions::Session};

/// The longest address of a text.
const MAX_URL: usize = 2000;
/// The longest name of a version of the terms.
pub const MAX_VERSION: usize = 64;

/// Where the operator's texts are, and which version of the terms counts.
///
/// `GET /api/login/ways` answers with it, to anybody.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS, ToSchema)]
#[ts(rename = "LegalLinks")]
#[schema(as = LegalLinks)]
pub struct Legal {
    pub imprint: Option<String>,
    pub privacy: Option<String>,
    pub terms: Option<String>,
    /// The version of the terms an account has to agree to; none if nobody
    /// has to.
    pub terms_version: Option<String>,
}

impl Legal {
    /// The texts the environment names.
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// The texts `var` names. An address that is not a whole `http` or `https`
    /// one, or a version without terms to agree to, stops the start.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let address = |name: &str| -> anyhow::Result<Option<String>> {
            let Some(value) = var(name).map(|value| value.trim().to_owned()) else {
                return Ok(None);
            };
            if value.is_empty() {
                return Ok(None);
            }
            let url = url::Url::parse(&value)
                .map_err(|err| anyhow::anyhow!("{name} is no address: {err}"))?;
            anyhow::ensure!(
                matches!(url.scheme(), "http" | "https") && url.host().is_some(),
                "{name} must be an http or https address"
            );
            anyhow::ensure!(value.len() <= MAX_URL, "{name} is longer than {MAX_URL}");
            Ok(Some(url.to_string()))
        };
        let legal = Self {
            imprint: address("LEGAL_IMPRINT_URL")?,
            privacy: address("LEGAL_PRIVACY_URL")?,
            terms: address("LEGAL_TERMS_URL")?,
            terms_version: var("TERMS_VERSION")
                .map(|version| version.trim().to_owned())
                .filter(|version| !version.is_empty()),
        };
        if let Some(version) = &legal.terms_version {
            anyhow::ensure!(
                legal.terms.is_some(),
                "TERMS_VERSION needs LEGAL_TERMS_URL: there is nothing to agree to"
            );
            anyhow::ensure!(
                version.chars().count() <= MAX_VERSION && version.chars().all(|c| !c.is_control()),
                "TERMS_VERSION must be a line of at most {MAX_VERSION} characters"
            );
        }
        Ok(legal)
    }

    /// The version an account has yet to agree to, when it agreed to `agreed`.
    pub fn to_accept(&self, agreed: Option<&str>) -> Option<String> {
        let wanted = self.terms_version.as_deref()?;
        (agreed != Some(wanted)).then(|| wanted.to_owned())
    }
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(accept))
}

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<Legal>().add::<Agreement>();
}

/// The version of the terms the reader agrees to.
///
/// `PUT /api/terms` reads it.
#[derive(Deserialize, TS, ToSchema)]
#[ts(rename = "TermsAgreement")]
#[schema(as = TermsAgreement)]
pub struct Agreement {
    version: String,
}

/// Notes that the account agreed to the version of the terms that counts now.
/// Another version is 409: the terms changed while the page asked.
#[utoipa::path(
    put,
    path = "/terms",
    tag = "Legal",
    request_body = Agreement,
    security(("session" = [])),
    responses((status = NO_CONTENT, description = "Done."), crate::openapi::NeedsSession, (status = NOT_FOUND, description = "The operator set no terms."), (status = CONFLICT, description = "Another version of the terms counts now.")),
)]
async fn accept(
    State(state): State<AppState>,
    session: Session,
    Json(agreement): Json<Agreement>,
) -> Result<StatusCode, AppError> {
    let Some(wanted) = state.legal.terms_version.as_deref() else {
        return Err(AppError::NotFound);
    };
    if agreement.version != wanted {
        let message = "these are not the terms that count now";
        return Err(AppError::Status(StatusCode::CONFLICT, message.into()));
    }
    sqlx::query!(
        "UPDATE accounts SET terms_version = $2, terms_accepted_at = now() WHERE id = $1",
        session.account_id,
        wanted
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Notes the agreement that came with a registration.
pub async fn agreed(db: &sqlx::PgPool, account_id: i32, version: &str) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE accounts SET terms_version = $2, terms_accepted_at = now() WHERE id = $1",
        account_id,
        version
    )
    .execute(db)
    .await?;
    Ok(())
}
