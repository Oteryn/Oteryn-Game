//! Crystal 201189f8 movements_unlock_entrace.lua:93-111.
//! The existing Quest writer has locked every participating Character root.
use super::*;
use oteryn_gameplay_quests::quest::{QuestTransition, world_callbacks::PartyRite};

struct Entry {
    character: CharacterId,
    revision: u64,
    changes: Vec<QuestTrackChange>,
    state_pin: Option<String>,
}

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    initiator: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    catalogue: &QuestStateCatalogue,
    transition: &QuestTransition,
    rite: &PartyRite,
    context: &crate::durability::quest_world_callback::QuestWorldContext,
) -> StoredResult<bool> {
    let keys: Vec<_> = transition.effects.iter().map(|effect| effect.track.as_str()).collect();
    let Some(hash) = catalogue.definition_hash(&transition.quest) else {
        return Ok(false);
    };
    let mut entries = Vec::new();
    for member in &context.party {
        let character = CharacterId::from_bytes(member.character)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let id = character.as_bytes().as_slice();
        let tracks = sqlx::query(
            "SELECT track_key,quest_key,value FROM game_character_quest_tracks
             WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[])
             ORDER BY track_key FOR UPDATE",
        ).bind(id).bind(&keys).fetch_all(&mut **tx).await?;
        let mut stored = BTreeMap::new();
        for row in &tracks {
            if row.try_get::<String,_>("quest_key")? != transition.quest {
                return Ok(false);
            }
            stored.insert(row.try_get("track_key")?, row.try_get("value")?);
        }
        // Both source gates are literal comparisons. No wall clock participates.
        let Ok(changes) = catalogue.evaluate(transition, &stored, 0) else {
            return Ok(false);
        };
        let Some(entry) = changes.iter().find(|change| change.track == rite.entry_track) else {
            return Ok(false);
        };
        if member.entry_before != Some(entry.before) {
            return Ok(false);
        }
        if entry.before != 1 {
            let crystal: Option<String> = sqlx::query_scalar(
                "SELECT i.item_instance_id::text
                 FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
                 JOIN game_item_instances i ON i.item_instance_id=c.item
                 WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0
                   AND i.definition_family='Item' AND i.definition_production_key=$3
                 ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i",
            ).bind(id).bind(initiator.runtime_scope.world_id().as_bytes().as_slice())
                .bind(&rite.admission_item).fetch_optional(&mut **tx).await?;
            if crystal.is_none() {
                return Ok(false);
            }
        }
        if character == initiator.character_id || entry.before == 1 {
            continue;
        }
        if !super::super::authored_admission::prerequisites_hold(tx, id, catalogue, &transition.quest).await?
            || !super::super::capacity::tracks_fit(tx, character, &keys, stored.len()).await?
        {
            return Ok(false);
        }
        let state = sqlx::query(
            "SELECT pinned_content_revision,definition_hash FROM game_character_quest_states
             WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE",
        ).bind(id).bind(&transition.quest).fetch_optional(&mut **tx).await?;
        let state_pin = if let Some(state) = state {
            if state.try_get::<Vec<u8>,_>("definition_hash")?.as_slice() != hash {
                return Ok(false);
            }
            Some(state.try_get::<String,_>("pinned_content_revision")?)
        } else {
            if !super::super::capacity::new_state_fits(tx, character).await? {
                return Ok(false);
            }
            None
        };
        let (revision, content): (String,String) = sqlx::query_as(
            "SELECT character_revision::text,content_revision FROM game_character_roots
             WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id).fetch_one(&mut **tx).await?;
        if content != catalogue.content_revision() {
            return Ok(false);
        }
        entries.push(Entry {
            character,
            revision: revision.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
            changes,
            state_pin,
        });
    }
    let world_binding = serde_json::to_vec(&(context, rite))
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    for entry in &entries {
        let mut binding = request_binding(entry.character, request);
        let digest = Sha256::new().chain_update(binding).chain_update(&world_binding).finalize();
        binding[1..].copy_from_slice(&digest);
        super::super::persist::persist(tx, super::super::persist::QuestWrite {
            character: entry.character,
            root_revision: entry.revision,
            request,
            transition,
            pin: entry.state_pin.as_deref().unwrap_or(catalogue.content_revision()),
            hash: &hash,
            changes: &entry.changes,
            binding: &binding,
            state_exists: entry.state_pin.is_some(),
        }).await?;
    }
    Ok(true)
}
