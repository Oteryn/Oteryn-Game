//! Historical appearance storage reads and recovery verification. Returned data is not
//! current actor, entitlement, sex, content-compatibility or publication authority.

use crate::domain::appearance::AppearanceSelection;
use crate::domain::{AccountId, CharacterId, CharacterRevision};
use sqlx::Row;
use sqlx::postgres::{PgConnection, PgRow};

#[derive(Debug)]
pub enum AppearanceReadError {
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
pub struct DurableAppearanceSelection {
    pub selection: AppearanceSelection<String>,
    pub content_revision: String,
    pub committed_character_revision: CharacterRevision,
}

/// An earned, append-only account fact. Store entitlements are deliberately absent.
/// Compatibility with a world's active definition must be independently resolved from content;
/// neither a matching key nor this recorded revision alone proves compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAppearanceUnlock {
    pub unlock_key: String,
    pub content_revision: String,
    pub granting_character_id: CharacterId,
    pub source_kind: String,
    pub source_event_id: Vec<u8>,
}

/// Read inside the caller's semantic transaction/admission load. This function neither checks
/// nor grants current gameplay authority. An absent selection stays absent, rather than storing
/// a guessed starter/fallback. Migration 0093 installs the table together with the recovery integration.
pub async fn read_selected_appearance(
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
/// from a request. Only the at-most-4096 declared `grant_keys` are read; the account/key
/// primary key bounds the result to that input count. Facts survive deletion of the granting Character; the private schema must
/// not cascade from that Character. The left join fails closed on a missing provenance receipt.
pub async fn read_account_appearance_unlocks(
    connection: &mut PgConnection,
    account: AccountId,
    grant_keys: &[String],
) -> Result<Vec<AccountAppearanceUnlock>, AppearanceReadError> {
    if grant_keys.len() > 4096 {
        return Err(AppearanceReadError::InvalidStoredState);
    }
    let rows = sqlx::query(
        "SELECT f.unlock_key, r.content_revision, r.granting_character_id::text, \
                r.source_kind, r.source_event_id \
           FROM game_account_appearance_unlocks f \
           LEFT JOIN game_account_appearance_unlock_receipts r \
             ON r.grant_occurrence_id=f.grant_occurrence_id \
            AND r.account_id=f.account_id AND r.unlock_key=f.unlock_key \
          WHERE f.account_id=encode($1,'hex')::uuid AND f.unlock_key=ANY($2::text[]) ORDER BY f.unlock_key COLLATE \"C\"",
    )
    .bind(account.as_bytes().as_slice())
    .bind(grant_keys)
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

pub(super) fn decode_selection_fields(
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

pub(super) fn valid_provenance(revision: &str) -> bool {
    !revision.is_empty() && revision.len() <= 128
}

/// APP-1's own recovery arm. The CP slot calls this from `verify_character_integrity` and
/// adds appearance receipts to its shared global-revision union, preserving the other families.
/// This validates historical storage only; it cannot issue gameplay authority.
pub async fn verify_character_appearance_integrity(
    connection: &mut PgConnection,
) -> Result<(), AppearanceReadError> {
    let invalid = sqlx::query(
        "WITH ordered AS ( \
           SELECT a.*, lag(outfit_after) OVER w AS previous_outfit, \
             lag(colours_after) OVER w AS previous_colours, \
             lag(addons_after) OVER w AS previous_addons, \
             lag(mount_after) OVER w AS previous_mount, \
             lag(content_revision) OVER w AS previous_content, \
             lag(committed_character_revision) OVER w AS previous_revision, \
             row_number() OVER (PARTITION BY character_id ORDER BY committed_character_revision DESC) AS latest \
           FROM game_character_appearance_receipts a \
           WINDOW w AS (PARTITION BY character_id ORDER BY committed_character_revision) \
         ) SELECT 1 FROM ordered a WHERE \
           (outfit_before,colours_before,addons_before,mount_before,content_before,selection_revision_before) \
             IS DISTINCT FROM \
           (previous_outfit,previous_colours,previous_addons,previous_mount,previous_content,previous_revision) \
           OR (latest=1 AND NOT EXISTS ( \
             SELECT 1 FROM game_character_appearance p WHERE p.character_id=a.character_id \
               AND (p.outfit_key,p.colours,p.addons,p.mount_key,p.content_revision, \
                    p.committed_character_revision,p.last_appearance_occurrence_id) \
                 IS NOT DISTINCT FROM \
                   (a.outfit_after,a.colours_after,a.addons_after,a.mount_after,a.content_revision, \
                    a.committed_character_revision,a.appearance_occurrence_id))) \
         UNION ALL SELECT 1 FROM game_character_appearance p \
           WHERE NOT EXISTS (SELECT 1 FROM ordered a WHERE a.character_id=p.character_id) \
         LIMIT 1",
    ).fetch_optional(connection).await?;
    if invalid.is_some() {
        return Err(AppearanceReadError::InvalidStoredState);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
