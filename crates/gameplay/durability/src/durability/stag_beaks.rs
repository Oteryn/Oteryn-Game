//! M41: bounded self-skinning provenance; an inventory beak is never a quest credit.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::item_mint_audit::Type2Transaction;
use super::item_transfer::CurrentCharacterItemFence;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use std::collections::BTreeMap;

pub const HARTFORD: &str = "oteryn:npc.guard_captain_hartford";
pub const STAGE: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline";
pub const BEAKS: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/harpy_beaks";
type Result<T> = std::result::Result<T, DurabilityError>;

/// Advisory snapshot with a sealed subject. The NPC writer independently reloads it under its fence.
#[derive(Debug)]
pub struct StagBeakPreview {
    character: CharacterId,
    count: u8,
}
impl StagBeakPreview {
    pub fn overlay(
        &self,
        character: CharacterId,
        tracks: &mut BTreeMap<String, i64>,
    ) -> Result<()> {
        if character != self.character {
            return Err(DurabilityError::InvalidStoredState);
        }
        // A stored inventory-era counter or a snapshot from another phase is not authority.
        let count = if tracks.get(STAGE) == Some(&6) {
            self.count
        } else {
            0
        };
        tracks.insert(BEAKS.into(), i64::from(count));
        Ok(())
    }
}

async fn count(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<u8> {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM (SELECT 1 FROM game_stag_beak_proofs p \
         JOIN game_character_quest_tracks q ON q.character_id=p.character_id \
          AND q.committed_character_revision=p.mission_revision \
         WHERE p.character_id=encode($1,'hex')::uuid AND q.track_key=$2 AND q.value=6 LIMIT 20) proven"
    ).bind(character.as_bytes().as_slice()).bind(STAGE).fetch_one(&mut **tx).await?;
    u8::try_from(count).map_err(|_| DurabilityError::InvalidStoredState)
}

pub(super) async fn overlay_locked(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    tracks: &mut BTreeMap<String, i64>,
) -> Result<()> {
    StagBeakPreview {
        character,
        count: count(tx, character).await?,
    }
    .overlay(character, tracks)
}

/// Participant of the already fully fenced craft transaction, after the real yield is minted.
/// The SQL trigger independently ties this row to that same transaction's immutable corpse proof.
pub(super) async fn record(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    transaction: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO game_stag_beak_proofs(craft_transaction_id,corpse_item_instance_id,character_id,mission_revision) \
         SELECT c.transaction_id,s.corpse_item_instance_id,c.character_id,q.committed_character_revision \
         FROM game_item_craft_records c JOIN game_corpse_skinning_records s USING(transaction_id) \
         JOIN game_corpse_source_races r ON r.item_instance_id=s.corpse_item_instance_id \
         JOIN game_character_quest_tracks q ON q.character_id=c.character_id \
         WHERE c.transaction_id=$1::uuid AND c.character_id=encode($2,'hex')::uuid \
          AND q.track_key=$3 AND q.value=6 \
          AND r.race_key IN ('oteryn:creature.night_harpy','oteryn:creature.day_night_harpy') \
          AND s.spent_key='oteryn:item.tibia.i52677' \
          AND jsonb_array_length(c.outputs)=1 AND c.outputs->0->>'key'='oteryn:item.tibia.i52632' \
          AND (SELECT count(*) FROM game_stag_beak_proofs p WHERE p.character_id=c.character_id \
               AND p.mission_revision=q.committed_character_revision)<20"
    ).bind(transaction).bind(fence.character_id.as_bytes().as_slice()).bind(STAGE)
        .execute(&mut ***tx).await?;
    Ok(())
}

impl DurabilityRoot {
    pub async fn read_stag_beak_preview(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<StagBeakPreview> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let count = count(&mut tx, character).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(StagBeakPreview { character, count })
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn character(last: u8) -> std::result::Result<CharacterId, String> {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = last;
        CharacterId::from_bytes(bytes).map_err(|e| format!("{e:?}"))
    }
    #[test]
    fn m41_preview_is_subject_bound_and_replaces_unproven_counters()
    -> std::result::Result<(), String> {
        let a = character(1)?;
        let b = character(2)?;
        let proof = StagBeakPreview {
            character: a,
            count: 0,
        };
        let mut tracks = BTreeMap::from([(STAGE.into(), 6), (BEAKS.into(), 999)]);
        let before = tracks.clone();
        assert!(proof.overlay(b, &mut tracks).is_err());
        assert_eq!(tracks, before, "wrong subject cannot alter a preview");
        proof
            .overlay(a, &mut tracks)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            tracks[BEAKS], 0,
            "possession or a legacy counter is not self-skinning provenance"
        );
        Ok(())
    }
    #[test]
    fn m41_past_phase_proof_cannot_authorize_a_new_phase() -> std::result::Result<(), String> {
        let a = character(1)?;
        let proof = StagBeakPreview {
            character: a,
            count: 20,
        };
        for stage in [0, 5, 7, 13] {
            let mut tracks = BTreeMap::from([(STAGE.into(), stage), (BEAKS.into(), 999)]);
            proof
                .overlay(a, &mut tracks)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(tracks[BEAKS], 0);
        }
        Ok(())
    }
}
