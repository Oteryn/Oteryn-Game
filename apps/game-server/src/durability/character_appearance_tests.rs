//! Single native SQLx implementation of historical appearance reads and recovery checks.
//!
//! This allocated filename also carries its focused tests. The native writer loads this exact
//! source as `stored`; the standalone PostgreSQL target loads the same source directly. Only
//! the tests below are conditional: the writer remains unconditional in `character_appearance.rs`.

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
pub(crate) async fn verify_character_appearance_integrity(
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
mod tests {
    use super::*;
    use crate::domain::appearance::{
        AccountUnlocks, AppearanceContent, AppearanceRejection, Sex, UnlockFact, displayed_look,
        validate_selection,
    };

    // Test-only catalogue: keys, sex and compatibility never come from a production request.
    struct Content;
    impl AppearanceContent<String> for Content {
        fn palette_size(&self) -> u16 {
            133
        }
        fn outfit_addon_count(&self, key: &String) -> Option<u8> {
            matches!(key.as_str(), "starter" | "earned" | "store").then_some(2)
        }
        fn outfit_is_starter(&self, key: &String, sex: Sex) -> bool {
            key == "starter" && sex == Sex::Male
        }
        fn addon_is_starter(&self, _: &String, _: Sex, _: u8) -> bool {
            false
        }
        fn mount_exists(&self, key: &String) -> bool {
            key == "horse"
        }
    }
    struct NoUnlocks;
    impl AccountUnlocks<String> for NoUnlocks {
        fn outfit(&self, _: &String) -> UnlockFact {
            UnlockFact::Absent
        }
        fn addon(&self, _: &String, _: u8) -> UnlockFact {
            UnlockFact::Absent
        }
        fn mount(&self, _: &String) -> UnlockFact {
            UnlockFact::Absent
        }
    }

    fn stored() -> DurableAppearanceSelection {
        decode_selection_fields(
            "starter".into(),
            &[0, 1, 2, 132],
            0,
            None,
            "content-1".into(),
            "2",
        )
        .expect("valid fixture")
    }

    #[test]
    fn preserves_unknown_keys_and_provenance_without_reinterpreting() {
        let selection = decode_selection_fields(
            "gone".into(),
            &[0, 1, 2, 132],
            0,
            Some("gone-mount".into()),
            "old-world".into(),
            "9",
        )
        .expect("historical selection need not exist in active content");
        assert_eq!(selection.selection.outfit_key, "gone");
        assert_eq!(selection.selection.mount_key.as_deref(), Some("gone-mount"));
        assert_eq!(selection.content_revision, "old-world");
        let fallback = stored();
        assert_eq!(
            displayed_look(
                &selection.selection,
                &fallback.selection,
                Sex::Male,
                &Content,
                &NoUnlocks
            )
            .outfit_key,
            "starter"
        );
        assert_eq!(selection.content_revision, "old-world");
        assert_eq!(selection.selection.outfit_key, "gone");
    }

    #[test]
    fn decoding_rejects_lossy_numbers_invalid_revisions_and_byte_overflow() {
        for (colours, addons, revision, provenance) in [
            (vec![0, 0, 0], 0, "2", "ok".to_owned()),
            (vec![-1, 0, 0, 0], 0, "2", "ok".to_owned()),
            (vec![65536, 0, 0, 0], 0, "2", "ok".to_owned()),
            (vec![0, 0, 0, 0], -1, "2", "ok".to_owned()),
            (vec![0, 0, 0, 0], 256, "2", "ok".to_owned()),
            (vec![0, 0, 0, 0], 0, "0", "ok".to_owned()),
            (vec![0, 0, 0, 0], 0, "18446744073709551616", "ok".to_owned()),
            (vec![0, 0, 0, 0], 0, "2", String::new()),
            (vec![0, 0, 0, 0], 0, "2", "é".repeat(65)),
        ] {
            assert!(matches!(
                decode_selection_fields(
                    "starter".into(),
                    &colours,
                    addons,
                    None,
                    provenance,
                    revision
                ),
                Err(AppearanceReadError::InvalidStoredState)
            ));
        }
        assert!(
            decode_selection_fields(
                "old".into(),
                &[65535; 4],
                255,
                None,
                "é".repeat(64),
                "18446744073709551615"
            )
            .is_ok()
        );
    }

    #[test]
    fn native_validator_rejects_each_locked_component_and_palette_boundary() {
        let mut value = stored().selection;
        assert_eq!(
            validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
            Ok(())
        );
        value.outfit_key = "earned".into();
        assert_eq!(
            validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
            Err(AppearanceRejection::OutfitNotAuthorized)
        );
        value.outfit_key = "store".into();
        assert_eq!(
            validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
            Err(AppearanceRejection::OutfitNotAuthorized)
        );
        value.outfit_key = "starter".into();
        value.addons = 1;
        assert_eq!(
            validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
            Err(AppearanceRejection::AddonNotAuthorized { addon: 1 })
        );
        value.addons = 4;
        assert_eq!(
            validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
            Err(AppearanceRejection::AddonOutOfRange { addon: 3 })
        );
        value.addons = 0;
        value.mount_key = Some("horse".into());
        assert_eq!(
            validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
            Err(AppearanceRejection::MountNotAuthorized)
        );
        value.mount_key = None;
        value.colours[3] = 133;
        assert_eq!(
            validate_selection(&value, Sex::Male, &Content, &NoUnlocks),
            Err(AppearanceRejection::ColourOutOfPalette { slot: 3 })
        );
    }
}
