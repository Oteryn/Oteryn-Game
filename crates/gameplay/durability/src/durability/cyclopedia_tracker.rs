//! P22: durable Cyclopedia Bestiary/Bosstiary tracker lists (migration 0586).
//!
//! Crystal keeps both sets on the player (player.cpp:820-865) and saves them as blobs
//! (iologindata_save_player.cpp:523, :1014); here each tracked creature is one row, so the
//! list survives logout and restart. A Bosstiary change needs a credited kill of that boss,
//! for add and remove alike (protocolgame.cpp:2790). The tracker is not progression: it moves
//! no CharacterRevision and uses the item-only fence, whose root lock serializes writers.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_item_only_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::cyclopedia_tracker::{TrackerAction, snapshot_fits};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackerKind {
    Bestiary = 1,
    Bosstiary = 2,
}

/// One tracked creature and its stored kill count (0 before the first credited kill).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackedCreature {
    pub creature_key: String,
    pub kill_count: u64,
}

const LIST: &str = "SELECT t.creature_key, COALESCE(b.kill_count, s.kill_count, 0) \
    FROM game_character_cyclopedia_tracker t \
    LEFT JOIN game_character_bestiary_progress b ON $2 = 1 AND b.character_id = t.character_id AND b.race_key = t.creature_key \
    LEFT JOIN game_character_bosstiary_progress s ON $2 = 2 AND s.character_id = t.character_id AND s.boss_key = t.creature_key \
    WHERE t.character_id = encode($1,'hex')::uuid AND t.tracker_kind = $2 \
    ORDER BY t.creature_key LIMIT $3";

impl DurabilityRoot {
    /// Applies `change` (or only reads, when `None`) and returns the resulting list. `Ok(None)`
    /// is a refusal: a stale fence, a Bosstiary change before the boss's first credited kill,
    /// or an add past the wire bound. A refusal writes nothing.
    pub async fn cyclopedia_tracker(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        kind: TrackerKind,
        change: Option<(TrackerAction, &str)>,
    ) -> Result<Option<Vec<TrackedCreature>>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        let change = change.map(|(action, key)| (action, key.to_owned()));
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(None);
            }
            let character = fence.character_id.as_bytes().as_slice();
            if let Some((action, key)) = &change {
                if kind == TrackerKind::Bosstiary {
                    let killed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 AND kill_count>0)")
                        .bind(character).bind(key).fetch_one(&mut *tx).await?;
                    if !killed {
                        return Ok(None);
                    }
                }
                match action {
                    TrackerAction::Add => {
                        sqlx::query("INSERT INTO game_character_cyclopedia_tracker(character_id,tracker_kind,creature_key) VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
                            .bind(character).bind(kind as i16).bind(key).execute(&mut *tx).await?;
                        let (count, key_bytes): (i64, i64) = sqlx::query_as("SELECT count(*), COALESCE(sum(octet_length(creature_key)),0)::bigint FROM game_character_cyclopedia_tracker WHERE character_id=encode($1,'hex')::uuid AND tracker_kind=$2")
                            .bind(character).bind(kind as i16).fetch_one(&mut *tx).await?;
                        let fits = usize::try_from(count).ok().zip(usize::try_from(key_bytes).ok())
                            .is_some_and(|(count, key_bytes)| snapshot_fits(count, key_bytes));
                        if !fits {
                            return Ok(None);
                        }
                    }
                    TrackerAction::Remove => {
                        sqlx::query("DELETE FROM game_character_cyclopedia_tracker WHERE character_id=encode($1,'hex')::uuid AND tracker_kind=$2 AND creature_key=$3")
                            .bind(character).bind(kind as i16).bind(key).execute(&mut *tx).await?;
                    }
                }
            }
            let limit = i64::try_from(oteryn_protocol_oteryn::cyclopedia_tracker::MAX_TRACKED)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let rows: Vec<(String, i64)> = sqlx::query_as(LIST)
                .bind(character).bind(kind as i16).bind(limit).fetch_all(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            rows.into_iter()
                .map(|(creature_key, kill_count)| {
                    let kill_count = u64::try_from(kill_count).map_err(|_| DurabilityError::InvalidStoredState)?;
                    Ok(TrackedCreature { creature_key, kill_count })
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Some)
        })).await
    }
}
