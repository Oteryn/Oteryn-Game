//! W06: first-species Echo Warden earnings belong to the existing Charm owner.
//!
//! Crystal `201189f` `src/io/iobestiary.cpp:408-478` grants major Charm Points once per
//! character and base species. The kill caller supplies captured damage participation;
//! this owner proves current character authority and retains the exact occurrence result.

use super::*;
use crate::domain::echo_warden::first_species_charm_points;
use crate::durability::bestiary_progress::BestiaryKillOccurrence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EchoWardenRewardOutcome {
    pub first_species: bool,
    pub points: u32,
    pub replayed: bool,
}

impl DurabilityRoot {
    /// Credit one captured Warden death while holding the character's revision slot.
    /// As for Bosstiary counters, the root lock serializes the earning with Charm spends;
    /// this monotonic earning does not advance CharacterRevision. Historical replay never
    /// grants live authority and returns its original first-species decision.
    pub async fn credit_echo_warden_kill(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: BestiaryKillOccurrence,
        species: &str,
        bestiary_stars: u8,
    ) -> std::result::Result<EchoWardenRewardOutcome, CharacterProgressionError> {
        let species = BestiaryRaceKey::new(species.to_owned())
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        if fence.character_lease_generation == 0 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    // One occurrence may be retried across reconnects, but never rebound to
                    // a different species or reward table. The advisory key precedes root locks.
                    sqlx::query(
                        "SELECT pg_advisory_xact_lock(hashtextextended(\
                         'oteryn:echo-warden:' || encode($1,'hex') || ':' || encode($2,'hex'),0))",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(occurrence.as_bytes().as_slice())
                    .execute(&mut *tx)
                    .await?;
                    let previous = sqlx::query(
                        "SELECT species_key,bestiary_stars,first_species,charm_points \
                           FROM game_character_echo_warden_kill_receipts \
                          WHERE character_id=encode($1,'hex')::uuid \
                            AND occurrence_id=encode($2,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(occurrence.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    if let Some(row) = previous {
                        if !same_reward_semantics(
                            row.try_get("species_key")?,
                            row.try_get("bestiary_stars")?,
                            species.as_str(),
                            bestiary_stars,
                        ) {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        let outcome = EchoWardenRewardOutcome {
                            first_species: row.try_get("first_species")?,
                            points: u32::try_from(row.try_get::<i16, _>("charm_points")?)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                            replayed: true,
                        };
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(outcome));
                    }
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let already_known: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM game_character_echo_warden_rewards \
                          WHERE character_id=encode($1,'hex')::uuid AND species_key=$2)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(species.as_str())
                    .fetch_one(&mut *tx)
                    .await?;
                    let first_species = !already_known;
                    let points = if first_species {
                        let count: i64 = sqlx::query_scalar(
                            "SELECT count(*) FROM (SELECT 1 \
                               FROM game_character_echo_warden_rewards \
                              WHERE character_id=encode($1,'hex')::uuid LIMIT $2) bounded",
                        )
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(i64::try_from(MAX_VIEW_RACES + 1)
                            .map_err(|_| DurabilityError::InvalidStoredState)?)
                        .fetch_one(&mut *tx)
                        .await?;
                        if usize::try_from(count)
                            .map_err(|_| DurabilityError::InvalidStoredState)?
                            >= MAX_VIEW_RACES
                        {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        }
                        first_species_charm_points(bestiary_stars)
                    } else {
                        0
                    };
                    sqlx::query(
                        "INSERT INTO game_character_echo_warden_kill_receipts \
                         (character_id,occurrence_id,species_key,bestiary_stars,first_species,charm_points) \
                         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(occurrence.as_bytes().as_slice())
                    .bind(species.as_str())
                    .bind(i16::from(bestiary_stars))
                    .bind(first_species)
                    .bind(i16::try_from(points).map_err(|_| DurabilityError::InvalidStoredState)?)
                    .execute(&mut *tx)
                    .await?;
                    if first_species {
                        sqlx::query(
                            "INSERT INTO game_character_echo_warden_rewards \
                             (character_id,species_key,first_occurrence_id,bestiary_stars,charm_points) \
                             VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,$5)",
                        )
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(species.as_str())
                        .bind(occurrence.as_bytes().as_slice())
                        .bind(i16::from(bestiary_stars))
                        .bind(i16::try_from(points)
                            .map_err(|_| DurabilityError::InvalidStoredState)?)
                        .execute(&mut *tx)
                        .await?;
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(EchoWardenRewardOutcome {
                        first_species,
                        points,
                        replayed: false,
                    }))
                })
            })
            .await?
    }
}

fn same_reward_semantics(
    stored_species: &str,
    stored_stars: i16,
    species: &str,
    stars: u8,
) -> bool {
    stored_species == species && stored_stars == i16::from(stars)
}

/// Both the Charm view and unlock decision call this under the same character root lock.
/// The rows are immutable earning evidence; no parallel spendable balance is materialized.
pub(super) async fn earned_points(
    connection: &mut PgConnection,
    character: CharacterId,
) -> std::result::Result<Vec<u32>, DurabilityError> {
    let points: Vec<i16> = sqlx::query_scalar(
        "SELECT charm_points FROM game_character_echo_warden_rewards \
          WHERE character_id=encode($1,'hex')::uuid ORDER BY species_key LIMIT $2",
    )
    .bind(character.as_bytes().as_slice())
    .bind(i64::try_from(MAX_VIEW_RACES + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
    .fetch_all(connection)
    .await?;
    if points.len() > MAX_VIEW_RACES {
        return Err(DurabilityError::InvalidStoredState);
    }
    points
        .into_iter()
        .map(|points| u32::try_from(points).map_err(|_| DurabilityError::InvalidStoredState))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_reward_replay_binds_species_and_difficulty() {
        assert!(same_reward_semantics("oteryn:creature.rat", 1, "oteryn:creature.rat", 1));
        assert!(!same_reward_semantics("oteryn:creature.rat", 1, "oteryn:creature.wolf", 1));
        assert!(!same_reward_semantics("oteryn:creature.rat", 1, "oteryn:creature.rat", 2));
        assert!(!same_reward_semantics("oteryn:creature.rat", -1, "oteryn:creature.rat", 255));
    }
}
