//! Consequences in the existing fenced victim death transaction, never a second commit.
use super::character_authority::ReconciledCharacterAuthority;
use super::character_death::{
    CharacterDeathRequest, CommittedCharacterDeath,
};
use super::character_progression::{CharacterProgressionError, CurrentCharacterGameplayFence};
use super::character_revision_sequencer::CharacterRevisionSequencer;
pub use super::pvp_death_request::{PvpContributorKind, PvpDeathContributor};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::foundation::WorldId;
use oteryn_gameplay_combat_rules::pvp::skull::{
    KILL_BLOCK_US, LEDGER_WINDOW_US, PvpSkull, contributor_points,
};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;

impl DurabilityRoot {
    /// The existing sequencer owns initialization, replay and pending-respawn recovery.
    pub async fn settle_player_pvp_death<const N: usize>(
        &self,
        sequencer: &CharacterRevisionSequencer,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
    ) -> Result<CommittedCharacterDeath> {
        self.settle_player_death(sequencer, authority, node, fence, request)
            .await
    }
}

/// Missing state rows are inserted in the same order as existing rows are locked.
/// The ledger guard also takes its victim state before any ledger tuple.
/// The death writer already holds the admission relation EXCLUSIVE fence before
/// its victim root, so missing-row foreign keys cannot invert concurrent deaths.
pub(super) async fn apply_pvp_death_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    victim: CharacterId,
    world: WorldId,
    occurrence: &[u8; 16],
    contributors: &[PvpDeathContributor],
) -> Result<()> {
    let mut characters = contributors
        .iter()
        .map(|c| c.character_id)
        .collect::<Vec<_>>();
    characters.push(victim);
    characters.sort_by_key(|c| *c.as_bytes());
    for character in characters {
        if !contributors.is_empty() {
            sqlx::query(
                "INSERT INTO game_character_pvp_state AS s (character_id, world_id) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid) \
             ON CONFLICT (character_id) DO NOTHING",
            )
            .bind(character.as_bytes().as_slice())
            .bind(world.as_bytes().as_slice())
            .execute(&mut **tx)
            .await
            .map_err(DurabilityError::from)?;
        }
        let correct_world: Option<bool> = sqlx::query_scalar(
            "SELECT world_id = encode($2,'hex')::uuid FROM game_character_pvp_state \
             WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
        )
        .bind(character.as_bytes().as_slice())
        .bind(world.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await
        .map_err(DurabilityError::from)?;
        if correct_world == Some(false) || (correct_world.is_none() && !contributors.is_empty()) {
            return Err(CharacterProgressionError::InvalidInput);
        }
    }
    let now_us: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp()) * 1000000)::bigint")
            .fetch_one(&mut **tx)
            .await
            .map_err(DurabilityError::from)?;
    let ledger = sqlx::query(
        "SELECT contributor_character_id::text AS contributor, compact_kind, \
         (extract(epoch FROM last_at) * 1000000)::bigint AS last_at_us \
         FROM game_character_pvp_ledger WHERE victim_character_id = encode($1,'hex')::uuid \
         AND last_at >= timestamptz 'epoch' + $2 * interval '1 microsecond' \
         ORDER BY contributor_character_id FOR UPDATE",
    )
    .bind(victim.as_bytes().as_slice())
    .bind(now_us - LEDGER_WINDOW_US)
    .fetch_all(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    if ledger.len() != contributors.len() {
        return Err(CharacterProgressionError::InvalidInput);
    }
    for (row, contributor) in ledger.iter().zip(contributors) {
        let id: String = row.try_get("contributor").map_err(DurabilityError::from)?;
        let kind: Option<i16> = row.try_get("compact_kind").map_err(DurabilityError::from)?;
        let expected_kind = match contributor.kind {
            PvpContributorKind::Damage => 1,
            PvpContributorKind::Assist => 2,
        };
        if row
            .try_get::<i64, _>("last_at_us")
            .map_err(DurabilityError::from)?
            > now_us
            || super::character_progression::uuid_text(&id)? != *contributor.character_id.as_bytes()
            || kind.is_some_and(|kind| kind != expected_kind)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
    }
    let damage_count = contributors
        .iter()
        .filter(|c| c.kind == PvpContributorKind::Damage)
        .count();
    for contributor in contributors {
        let increment = contributor_points(contributor.kind, damage_count);
        let skull = if contributor.unjustified {
            sqlx::query(
                "INSERT INTO game_character_unjustified_points \
                 (character_id, death_occurrence_id, victim_character_id, points_milli) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, $4)",
            )
            .bind(contributor.character_id.as_bytes().as_slice()).bind(occurrence.as_slice())
            .bind(victim.as_bytes().as_slice()).bind(i32::from(increment))
            .execute(&mut **tx).await.map_err(DurabilityError::from)?;
            sqlx::query(
                "INSERT INTO game_character_revenge_marks \
                 (victim_character_id, killer_character_id, death_occurrence_id, expires_at) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, clock_timestamp())",
            )
            .bind(victim.as_bytes().as_slice()).bind(contributor.character_id.as_bytes().as_slice())
            .bind(occurrence.as_slice()).execute(&mut **tx).await.map_err(DurabilityError::from)?;
            Some(
                super::pvp_skull_storage::evaluate_stored_skull(
                    tx,
                    contributor,
                    occurrence,
                    now_us,
                    increment,
                )
                .await?,
            )
        } else {
            None
        };
        let kill_until = now_us
            .checked_add(KILL_BLOCK_US)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let (skull_code, until) = skull.map_or((None, None), |state| {
            (
                Some(match state.skull {
                    PvpSkull::None => 0i16,
                    PvpSkull::Red => 1,
                    PvpSkull::Black => 2,
                }),
                state.until_us,
            )
        });
        sqlx::query(
            "UPDATE game_character_pvp_state SET \
             kill_block_until = GREATEST(kill_block_until, timestamptz 'epoch' + $2 * interval '1 microsecond'), \
             white_skull_until = CASE WHEN $3 THEN GREATEST(white_skull_until, kill_block_until, \
                 timestamptz 'epoch' + $2 * interval '1 microsecond') ELSE white_skull_until END, \
             skull = COALESCE($4, skull), \
             skull_until = CASE WHEN $4::smallint IS NULL THEN skull_until \
                 WHEN $5::bigint IS NULL THEN NULL ELSE timestamptz 'epoch' + $5 * interval '1 microsecond' END, \
             revision = revision + 1 WHERE character_id = encode($1,'hex')::uuid",
        )
        .bind(contributor.character_id.as_bytes().as_slice()).bind(kill_until)
        .bind(contributor.unjustified).bind(skull_code).bind(until)
        .execute(&mut **tx).await.map_err(DurabilityError::from)?;
    }
    sqlx::query(
        "DELETE FROM game_character_pvp_ledger WHERE victim_character_id = encode($1,'hex')::uuid",
    )
    .bind(victim.as_bytes().as_slice())
    .execute(&mut **tx)
    .await
    .map_err(DurabilityError::from)?;
    Ok(())
}
