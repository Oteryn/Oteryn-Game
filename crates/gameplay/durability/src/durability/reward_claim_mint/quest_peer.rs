//! Read-only peer predicates share the enclosing reward's authority and transaction.
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, character_progression::CurrentCharacterGameplayFence};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, reward_callbacks::TrackRequirement};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use sqlx::{Acquire, Row};

/// Issued after resolving an exact live player in the acting runtime owner.
#[derive(Clone, Copy, Debug)]
pub struct QuestPeerUseProof {
    pub fence: CurrentCharacterGameplayFence,
    pub target: EntityRef,
}

/// A successful read is authoritative only while the caller retains this same
/// transaction through its own reward commit. It grants no peer mutation right.
pub async fn holds_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    actor: CurrentCharacterGameplayFence,
    peer: &QuestPeerUseProof,
    requirement: &TrackRequirement,
    catalogue: &QuestStateCatalogue,
    carried: &super::QuestCarriedUse,
) -> Result<bool, DurabilityError> {
    if peer.fence.character_id == actor.character_id
        || peer.fence.game_session_id == actor.game_session_id
        || peer.fence.runtime_scope != actor.runtime_scope
        || peer.fence.scope_ownership_generation != actor.scope_ownership_generation
        || peer.target.generation == 0
        || requirement.minimum > requirement.maximum
    {
        return Ok(false);
    }
    let Ok(intent) =
        oteryn_protocol_oteryn::world_object::decode_use_intent_target(&[4, 15], &carried.intent)
    else {
        return Ok(false);
    };
    if intent.use_with != Some(peer.target)
        || intent.use_with_map_item.is_some()
        || intent.use_with_position.is_some()
    {
        return Ok(false);
    }
    let Some(track) = catalogue.track(&requirement.track) else {
        return Ok(false);
    };
    // The actor root is already locked. Refuse a busy peer rather than introduce
    // opposite-order root waits between simultaneous A->B and B->A commands.
    let mut probe = tx.begin().await?;
    let locked = async {
        let session = sqlx::query_scalar::<_, i32>("SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR SHARE NOWAIT")
            .bind(peer.fence.game_session_id.as_bytes().as_slice()).fetch_optional(&mut *probe).await?;
        if session.is_none() { return Ok(None); }
        sqlx::query_scalar::<_, i32>("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE NOWAIT")
            .bind(peer.fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *probe).await
    }.await;
    match locked {
        Ok(Some(_)) => probe.commit().await?,
        Ok(None) => {
            probe.rollback().await?;
            return Ok(false);
        }
        Err(error) => {
            let busy = error
                .as_database_error()
                .and_then(|e| e.code())
                .is_some_and(|code| code == "55P03");
            probe.rollback().await?;
            return if busy { Ok(false) } else { Err(error.into()) };
        }
    }
    let Ok(root) =
        crate::durability::character_progression::assert_gameplay_fence(tx, &peer.fence, node)
            .await?
    else {
        return Ok(false);
    };
    if root.content_revision != catalogue.content_revision() {
        return Ok(false);
    }
    // Departure can commit before runtime placement catches up. The retained
    // peer root lock serializes this read with every arrival-producing writer.
    let departing: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid)")
        .bind(peer.fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if departing {
        return Ok(false);
    }
    let pin = sqlx::query("SELECT definition_hash,pinned_content_revision FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(peer.fence.character_id.as_bytes().as_slice()).bind(&track.quest)
        .fetch_optional(&mut **tx).await?;
    if let Some(pin) = &pin {
        let expected = catalogue.definition_hash(&track.quest);
        if pin.try_get::<String, _>("pinned_content_revision")? != catalogue.content_revision()
            || expected.is_none_or(|expected| {
                pin.try_get::<Vec<u8>, _>("definition_hash").ok().as_deref()
                    != Some(expected.as_slice())
            })
        {
            return Ok(false);
        }
    }
    let row = sqlx::query("SELECT value,quest_key,committed_character_revision::text AS revision FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(peer.fence.character_id.as_bytes().as_slice()).bind(&requirement.track)
        .fetch_optional(&mut **tx).await?;
    let value = match row {
        Some(row) => {
            if pin.is_none() || row.try_get::<String, _>("quest_key")? != track.quest {
                return Ok(false);
            }
            let revision = row
                .try_get::<String, _>("revision")?
                .parse::<u64>()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if revision > root.revision {
                return Ok(false);
            }
            row.try_get::<i64, _>("value")?
        }
        None => track.initial,
    };
    Ok((track.min..=track.max).contains(&value)
        && (requirement.minimum..=requirement.maximum).contains(&value))
}

#[cfg(test)]
mod tests;
