//! The basin's authored progress participates in the same transaction as its
//! coin debit and durable placement.
use super::*;
use super::super::{
    character_last_position::{FinalLastPosition, LastPositionKey, LastPositionWrite, upsert_last_position},
    item_transfer::CurrentCharacterItemFence,
};

#[derive(Clone, Debug)]
pub struct CoalBasinArrival {
    pub fence: CurrentCharacterItemFence,
    pub position: FinalLastPosition,
    pub catalogue: Arc<QuestStateCatalogue>,
}

pub(in crate::durability) async fn settle(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    arrival: &CoalBasinArrival,
    uid: u16,
    transaction: [u8; 16],
) -> StoredResult<bool> {
    let fence = arrival.fence;
    let Ok(root) = super::super::character_progression::assert_item_only_fence(tx, &fence, node).await? else {
        return Ok(false);
    };
    let catalogue = &arrival.catalogue;
    if root.content_revision != catalogue.content_revision() {
        return Ok(false);
    }
    let position = arrival.position;
    let key = LastPositionKey {
        character_lease_generation: fence.character_lease_generation,
        write_sequence: position.write_sequence,
    };
    if upsert_last_position(tx, fence.character_id.as_bytes(), key, &position.record).await?
        != LastPositionWrite::Written {
        return Ok(false);
    }
    let mut revision = root.revision;
    let extra = match uid {
        9035 => Some("diphtrahs_treasure/other_2"),
        9036 => Some("mahrdis_treasure/other_1"),
        _ => None,
    };
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM transaction_timestamp()))::bigint")
        .fetch_one(&mut **tx).await?;
    for step in std::iter::once("default_start/movement_1").chain(extra) {
        let hash = Sha256::new()
            .chain_update(b"oteryn:coal-basin-progress:v1")
            .chain_update(transaction)
            .chain_update(step.as_bytes())
            .finalize();
        let ordinal = u64::from_be_bytes(hash[..8].try_into().map_err(|_| DurabilityError::InvalidStoredState)?).max(1);
        let command = CommandId::new(ordinal).map_err(|_| DurabilityError::InvalidStoredState)?;
        let request = QuestTransitionRequest {
            transition_key: format!("oteryn:quest-transition/quest/u7_4/the_ancient_tombs/{step}"),
            cause: QuestCause::Step(CommandRef::new(fence.game_session_id, command)),
        };
        let Some(next) = write_step(tx, fence.character_id, catalogue, revision, &request, now).await? else {
            return Ok(false);
        };
        revision = next;
    }
    Ok(true)
}

async fn write_step(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    catalogue: &QuestStateCatalogue,
    revision: u64,
    request: &QuestTransitionRequest,
    now: i64,
) -> StoredResult<Option<u64>> {
    let transition = catalogue.transition(&request.transition_key)
        .filter(|transition| !transition.completes && transition.experience.is_none()
            && transition.world_change.is_none())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let quest = transition.quest.as_str();
    let hash = catalogue.definition_hash(quest).ok_or(DurabilityError::InvalidStoredState)?;
    let state: Option<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT pinned_content_revision,definition_hash FROM game_character_quest_states
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE",
    ).bind(character.as_bytes().as_slice()).bind(quest).fetch_optional(&mut **tx).await?;
    let pin = match &state {
        Some((pin, saved)) if *saved == hash => pin,
        Some(_) => return Ok(None),
        None => catalogue.content_revision(),
    };
    let keys: Vec<_> = transition.effects.iter().map(|effect| effect.track.as_str()).collect();
    let found: Vec<(String, i64)> = sqlx::query_as(
        "SELECT track_key,value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND track_key=ANY($3)
         ORDER BY track_key FOR UPDATE",
    ).bind(character.as_bytes().as_slice()).bind(quest).bind(&keys).fetch_all(&mut **tx).await?;
    let tracks: BTreeMap<String, i64> = found.into_iter().collect();
    let changes = match catalogue.evaluate(transition, &tracks, now) {
        Ok(changes) => changes,
        Err(QuestRefusal::StageMismatch) => return Ok(Some(revision)),
        Err(_) => return Ok(None),
    };
    let changed: Vec<_> = changes.iter().map(|change| change.track.as_str()).collect();
    let present = changes.iter().filter(|change| tracks.contains_key(&change.track)).count();
    if (state.is_none() && !capacity::new_state_fits(tx, character).await?)
        || !capacity::tracks_fit(tx, character, &changed, present).await?
    {
        return Ok(None);
    }
    let binding = request_binding(character, request);
    persist::persist(tx, persist::QuestWrite {
        character, root_revision: revision, request, transition, pin, hash: &hash,
        changes: &changes, binding: &binding, state_exists: state.is_some(),
    }).await.map(Some)
}
