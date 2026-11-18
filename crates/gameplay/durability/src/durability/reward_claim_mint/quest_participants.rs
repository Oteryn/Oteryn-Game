//! Atomic, source-bound writes through existing foreign quest owners.
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::durability::{DurabilityError, character_progression::CurrentCharacterGameplayFence};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, npc_rules};
use oteryn_gameplay_quests::quest::reward_callbacks::{RewardCallback, participant_key};
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeMap;

fn accepts_definition_pin(
    catalogue: &QuestStateCatalogue,
    quest: &str,
    current: [u8; 32],
    stored: &[u8],
    completed: bool,
) -> bool {
    stored == current
        || completed
        || <&[u8; 32]>::try_from(stored)
            .is_ok_and(|hash| npc_rules::legacy_defaults(catalogue, quest, hash).is_some())
}

/// Character root and the sorted union of all source track locks are already held.
/// A false result rolls back the entire source transaction, including these writes.
pub(super) async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &RewardCallback,
    catalogue: &QuestStateCatalogue,
    reserve_primary_state: bool,
) -> Result<bool, DurabilityError> {
    if callback.participants.is_empty() {
        return Ok(true);
    }
    let QuestCause::Command(command) = request.cause else {
        return Ok(false);
    };
    let id = fence.character_id.as_bytes().as_slice();
    let revision = fence
        .expected_character_revision
        .get()
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?
        .to_string();
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&mut **tx)
        .await?;
    let mut participants: Vec<_> = callback.participants.iter().enumerate().collect();
    participants.sort_by_key(|(_, p)| &p.quest);
    for (index, participant) in participants {
        let transition = catalogue
            .transition(&participant_key(&callback.key, index))
            .ok_or(DurabilityError::InvalidStoredState)?;
        if transition.quest != participant.quest {
            return Err(DurabilityError::InvalidStoredState);
        }
        let current_hash = catalogue
            .definition_hash(&participant.quest)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let state = sqlx::query("SELECT pinned_content_revision,definition_hash,completed_character_revision IS NOT NULL AS completed FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE")
            .bind(id).bind(&participant.quest).fetch_optional(&mut **tx).await?;
        let (pin, hash) = if let Some(state) = &state {
            let hash: Vec<u8> = state.try_get("definition_hash")?;
            if !accepts_definition_pin(
                catalogue,
                &participant.quest,
                current_hash,
                &hash,
                state.try_get("completed")?,
            ) {
                return Ok(false);
            }
            (state.try_get::<String, _>("pinned_content_revision")?, hash)
        } else {
            // Include the still-pending primary state in this same admission.
            if !crate::durability::quest_state::capacity::states_fit(
                tx,
                fence.character_id,
                1 + usize::from(reserve_primary_state),
            )
            .await?
            {
                return Ok(false);
            }
            (
                catalogue.content_revision().to_owned(),
                current_hash.to_vec(),
            )
        };
        let mut keys: Vec<_> = participant.tracks().collect();
        keys.sort_unstable();
        let rows = sqlx::query("SELECT track_key,quest_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key FOR UPDATE")
            .bind(id).bind(&keys).fetch_all(&mut **tx).await?;
        let mut stored = BTreeMap::new();
        for row in rows {
            if row.try_get::<String, _>("quest_key")? != participant.quest {
                return Err(DurabilityError::InvalidStoredState);
            }
            stored.insert(
                row.try_get::<String, _>("track_key")?,
                row.try_get::<i64, _>("value")?,
            );
        }
        let Ok(changes) = catalogue.evaluate(transition, &stored, now) else {
            return Ok(false);
        };
        let track_keys: Vec<_> = changes.iter().map(|c| c.track.as_str()).collect();
        let before: Vec<_> = changes.iter().map(|c| c.before).collect();
        let after: Vec<_> = changes.iter().map(|c| c.after).collect();
        sqlx::query("INSERT INTO game_quest_reward_participants(character_id,cause_id,cause_ordinal,transition_key,participant_index,quest_key,plan,pinned_content_revision,definition_hash,track_keys,values_before,values_after,committed_character_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13::text::numeric)")
            .bind(id).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
            .bind(&callback.key).bind(i16::try_from(index).map_err(|_|DurabilityError::InvalidStoredState)?)
            .bind(&participant.quest).bind(serde_json::to_value(participant).map_err(|_|DurabilityError::InvalidStoredState)?)
            .bind(&pin).bind(&hash).bind(&track_keys).bind(&before).bind(&after).bind(&revision)
            .execute(&mut **tx).await?;
        for change in &changes {
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
                .bind(id).bind(&change.track).bind(&participant.quest).bind(change.after).bind(&revision)
                .execute(&mut **tx).await?;
        }
        if state.is_some() {
            sqlx::query("UPDATE game_character_quest_states SET committed_character_revision=$3::text::numeric WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
                .bind(id).bind(&participant.quest).bind(&revision).execute(&mut **tx).await?;
        } else {
            sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,$5::text::numeric)")
                .bind(id).bind(&participant.quest).bind(&pin).bind(&hash).bind(&revision).execute(&mut **tx).await?;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_quests::quest::loader;

    #[test]
    fn unfinished_participants_accept_only_current_or_exact_predecessor_pins() -> Result<(), String> {
        const OUTFIT: &str = "oteryn:quest.outfit_and_addon_quests";
        const ROOK: &str = "oteryn:quest.citizen_outfits_quest_on_rook";
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let current = catalogue.definition_hash(OUTFIT).ok_or("Outfit definition")?;
        let prior = [
            0xf0, 0x3f, 0xaa, 0x35, 0xab, 0xd9, 0xaf, 0x89, 0xc2, 0x81, 0xd3, 0xb4, 0xf7, 0xdb,
            0x2d, 0xd5, 0x69, 0xae, 0x32, 0xea, 0xdd, 0x91, 0x87, 0x72, 0x7e, 0x78, 0x82, 0xe4,
            0xc9, 0x0e, 0x3c, 0xcf,
        ];
        assert_ne!(current, prior);
        assert!(accepts_definition_pin(catalogue, OUTFIT, current, &current, false));
        assert!(accepts_definition_pin(catalogue, OUTFIT, current, &prior, false));
        let mut unknown = prior;
        unknown[0] ^= 1;
        assert!(!accepts_definition_pin(catalogue, OUTFIT, current, &unknown, false));
        assert!(!accepts_definition_pin(catalogue, OUTFIT, current, &prior[..31], false));
        assert!(accepts_definition_pin(catalogue, OUTFIT, current, &unknown, true));
        let rook_current = catalogue.definition_hash(ROOK).ok_or("Rook definition")?;
        assert!(!accepts_definition_pin(catalogue, ROOK, rook_current, &prior, false));
        Ok(())
    }
}
