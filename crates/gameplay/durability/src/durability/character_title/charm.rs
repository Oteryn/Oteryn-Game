//! P12a reads the Charm owner's earned and spent balance; it never stores a second balance.
use crate::content::charm_source::CanonicalCharmCatalogue;
use crate::content::{ProjectEvidenceLimits, canonical_bestiary_rows};
use crate::domain::CharacterId;
use crate::domain::charm::{CharmKey, CharmStage, derive_balance};
use crate::durability::DurabilityError;
use crate::durability::charm_state::{BestiaryCharmEntry, BestiaryCharmFacts, CharmFacts};
use sqlx::Row;
use std::collections::BTreeMap;
use std::sync::LazyLock;

pub(super) struct Source {
    facts: BestiaryCharmFacts,
    catalogue: CanonicalCharmCatalogue,
}

/// Decode the same canonical sources as the live Charm command before opening a transaction.
/// An unavailable source contributes no title qualification.
pub(super) fn source() -> Option<&'static Source> {
    static SOURCE: LazyLock<Option<Source>> = LazyLock::new(|| {
        let entries = canonical_bestiary_rows(ProjectEvidenceLimits {
            max_documents: 5,
            max_document_bytes: 2_500_000,
            max_total_bytes: 8_000_000,
            max_json_depth: 10,
            max_decoded_fields: 65_000,
            max_string_bytes: 1_000_000,
            max_locator_bytes: 128,
            max_locator_segments: 5,
            max_reference_records: 4096,
            max_import_records: 1,
            max_reimport_states: 1,
        })
        .ok()?;
        let facts = BestiaryCharmFacts::new(
            entries
                .into_iter()
                .map(|(race, charm_points)| BestiaryCharmEntry { race, charm_points }),
        )
        .ok()?;
        Some(Source {
            facts,
            catalogue: CanonicalCharmCatalogue::embedded().ok()?,
        })
    });
    SOURCE.as_ref()
}

impl Source {
    /// Crystal `player_title.cpp:312-314` asks for current, unspent Charm Points, not their
    /// lifetime total (`player.cpp:1779-1785`). The existing `derive_balance` owns subtraction.
    pub(super) async fn available(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character: CharacterId,
    ) -> Result<u64, DurabilityError> {
        let points = self
            .facts
            .completed_entry_charm_points(tx, character)
            .await?;
        let bound = oteryn_protocol_oteryn::charm::MAX_CHARMS;
        let rows = sqlx::query(
            "SELECT charm_key,unlocked_stage FROM game_character_charm_unlocks \
             WHERE character_id=encode($1,'hex')::uuid ORDER BY charm_key LIMIT $2",
        )
        .bind(character.as_bytes().as_slice())
        .bind(i64::try_from(bound + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
        .fetch_all(&mut **tx)
        .await?;
        if rows.len() > bound {
            return Err(DurabilityError::InvalidStoredState);
        }
        let mut unlocks = BTreeMap::new();
        for row in rows {
            let key = CharmKey::new(row.try_get::<String, _>("charm_key")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let stage = u8::try_from(row.try_get::<i16, _>("unlocked_stage")?)
                .ok()
                .and_then(|value| CharmStage::new(value).ok())
                .ok_or(DurabilityError::InvalidStoredState)?;
            unlocks.insert(key, stage);
        }
        // Promotion changes Minor Charm Echoes only, never the Charm Points predicate.
        let balance = derive_balance(self.catalogue.catalogue(), &unlocks, points, false)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        Ok(balance.points_earned.saturating_sub(balance.points_spent))
    }
}
