//! APP-1 private durable read shapes. No write or gameplay authority is issued here.
//!
//! The final selected-appearance event/receipt and earned-grant consumer require the CP schema
//! and shared revision/integrity slot. A read deliberately preserves unknown content keys and
//! provenance: the existing `domain::appearance` validator decides usability on the current
//! World, and `displayed_look` supplies fallback without rewriting the stored selection.

use crate::domain::appearance::AppearanceSelection;
use crate::domain::{AccountId, CharacterId, CharacterRevision};
use sqlx::Row;
use sqlx::postgres::{PgConnection, PgRow};

#[derive(Debug)]
pub(crate) enum AppearanceReadError {
    InvalidStoredState,
    Storage(sqlx::Error),
}

impl From<sqlx::Error> for AppearanceReadError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error)
    }
}

impl std::fmt::Display for AppearanceReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStoredState => formatter.write_str("invalid stored appearance"),
            Self::Storage(error) => write!(formatter, "appearance storage unavailable: {error}"),
        }
    }
}
impl std::error::Error for AppearanceReadError {}

/// Historical selection, never a current session, content-compatibility or entitlement proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableAppearanceSelection {
    pub(crate) selection: AppearanceSelection<String>,
    pub(crate) content_revision: String,
    pub(crate) committed_character_revision: CharacterRevision,
}

/// An earned, append-only account fact. Store entitlements are deliberately absent.
/// Compatibility with a world's active definition must be independently resolved from content;
/// neither a matching key nor this recorded revision alone proves compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccountAppearanceUnlock {
    pub(crate) unlock_key: String,
    pub(crate) content_revision: String,
    pub(crate) granting_character_id: CharacterId,
    pub(crate) source_kind: String,
    pub(crate) source_event_id: Vec<u8>,
}

/// Read inside the caller's semantic transaction/admission load. This function neither checks
/// nor grants current gameplay authority. An absent selection stays absent, rather than storing
/// a guessed starter/fallback. The private table is installed only in the CP migration slot.
pub(crate) async fn read_selected_appearance(
    connection: &mut PgConnection,
    character: CharacterId,
) -> Result<Option<DurableAppearanceSelection>, AppearanceReadError> {
    let row = sqlx::query(
        "SELECT outfit_key, colours, addons, mount_key, content_revision, \
                committed_character_revision::text \
           FROM game_character_appearance WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_optional(connection)
    .await?;
    row.as_ref().map(decode_selection).transpose()
}

/// Account identity belongs to the caller's authorized read context. A caller handling a grant
/// or selection mutation must derive it from the independently fenced Character event, never
/// from a request. Facts survive deletion of the granting Character; the private schema must
/// not cascade from that Character. The left join fails closed on a missing provenance receipt.
pub(crate) async fn read_account_appearance_unlocks(
    connection: &mut PgConnection,
    account: AccountId,
) -> Result<Vec<AccountAppearanceUnlock>, AppearanceReadError> {
    let rows = sqlx::query(
        "SELECT f.unlock_key, r.content_revision, r.granting_character_id::text, \
                r.source_kind, r.source_event_id \
           FROM game_account_appearance_unlocks f \
           LEFT JOIN game_account_appearance_unlock_receipts r \
             ON r.grant_occurrence_id=f.grant_occurrence_id \
            AND r.account_id=f.account_id AND r.unlock_key=f.unlock_key \
          WHERE f.account_id=encode($1,'hex')::uuid ORDER BY f.unlock_key COLLATE \"C\"",
    )
    .bind(account.as_bytes().as_slice())
    .fetch_all(connection)
    .await?;
    rows.iter().map(decode_unlock).collect()
}

fn decode_selection(row: &PgRow) -> Result<DurableAppearanceSelection, AppearanceReadError> {
    let colours: Vec<i32> = row.try_get("colours")?;
    let addons: i16 = row.try_get("addons")?;
    let revision: String = row.try_get("committed_character_revision")?;
    decode_selection_fields(
        row.try_get("outfit_key")?,
        &colours,
        addons,
        row.try_get("mount_key")?,
        row.try_get("content_revision")?,
        &revision,
    )
}

fn decode_selection_fields(
    outfit_key: String,
    colours: &[i32],
    addons: i16,
    mount_key: Option<String>,
    content_revision: String,
    revision: &str,
) -> Result<DurableAppearanceSelection, AppearanceReadError> {
    let invalid = || AppearanceReadError::InvalidStoredState;
    if !valid_provenance(&content_revision) {
        return Err(invalid());
    }
    let colours: [i32; 4] = colours.try_into().map_err(|_| invalid())?;
    let colours = [
        u16::try_from(colours[0]).map_err(|_| invalid())?,
        u16::try_from(colours[1]).map_err(|_| invalid())?,
        u16::try_from(colours[2]).map_err(|_| invalid())?,
        u16::try_from(colours[3]).map_err(|_| invalid())?,
    ];
    let revision = revision.parse().map_err(|_| invalid())?;
    Ok(DurableAppearanceSelection {
        selection: AppearanceSelection {
            outfit_key,
            colours,
            addons: u8::try_from(addons).map_err(|_| invalid())?,
            mount_key,
        },
        content_revision,
        committed_character_revision: CharacterRevision::new(revision).map_err(|_| invalid())?,
    })
}

fn decode_unlock(row: &PgRow) -> Result<AccountAppearanceUnlock, AppearanceReadError> {
    let content_revision: String = row.try_get("content_revision")?;
    let source_kind: String = row.try_get("source_kind")?;
    let source_event_id: Vec<u8> = row.try_get("source_event_id")?;
    let character: String = row.try_get("granting_character_id")?;
    // Decode through PostgreSQL's UUID wire type; do not introduce a second UUID parser.
    let character = sqlx::types::Uuid::parse_str(&character)
        .map_err(|_| AppearanceReadError::InvalidStoredState)?;
    if !valid_provenance(&content_revision) || source_kind.is_empty() || source_event_id.is_empty()
    {
        return Err(AppearanceReadError::InvalidStoredState);
    }
    Ok(AccountAppearanceUnlock {
        unlock_key: row.try_get("unlock_key")?,
        content_revision,
        granting_character_id: CharacterId::from_bytes(*character.as_bytes())
            .map_err(|_| AppearanceReadError::InvalidStoredState)?,
        source_kind,
        source_event_id,
    })
}

fn valid_provenance(revision: &str) -> bool {
    !revision.is_empty() && revision.len() <= 128
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "character_appearance_tests.rs"]
mod tests;
