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
        let mut points = self
            .facts
            .completed_entry_charm_points(tx, character)
            .await?;
        points.extend(crate::durability::charm_state::echo_warden::earned_points(tx, character).await?);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beastly_counts_warden_earnings() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
            tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
                use sqlx::Connection;
                let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()); };
                let mut connection = sqlx::PgConnection::connect(&url).await?;
                let mut tx = connection.begin().await?;
                sqlx::raw_sql("CREATE TEMP TABLE game_character_bestiary_progress(character_id uuid,race_key text,kill_count bigint); CREATE TEMP TABLE game_character_charm_unlocks(character_id uuid,charm_key text,unlocked_stage smallint); CREATE TEMP TABLE game_character_echo_warden_rewards(character_id uuid,species_key text,charm_points smallint);").execute(&mut *tx).await?;
                let character = CharacterId::from_bytes([1,2,3,4,5,6,0x70,8,0x80,10,11,12,13,14,15,1])?;
                sqlx::query("INSERT INTO game_character_echo_warden_rewards SELECT encode($1,'hex')::uuid,n::text,100 FROM generate_series(1,20) n").bind(character.as_bytes().as_slice()).execute(&mut *tx).await?;
                let source = source().ok_or("missing canonical charm source")?;
                assert_eq!(source.available(&mut tx, character).await?, 2000);
                Ok(())
            })
        })?.join().map_err(|_| "test thread panicked")?
    }
}
