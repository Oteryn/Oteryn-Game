//! Persist the ticket transition in the paying voyage's transaction.
use super::*;

pub(in crate::durability) async fn purchase(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    service: &str,
    catalogue: &QuestStateCatalogue,
) -> StoredResult<Option<CharacterRevision>> {
    const QUEST: &str = "oteryn:quest.travora_ticket";
    const TRACK: &str = "oteryn:quest-progress/travora/ticket_purchased";
    let Some(captain) = service.strip_prefix("oteryn:service.travel.") else {
        return Ok(None);
    };
    let key = format!("oteryn:quest-reward/travora/{captain}");
    let Some(transition) = catalogue.transition(&key) else {
        return Ok(None);
    };
    if transition.quest != QUEST
        || !matches!(transition.effects.as_slice(), [effect] if effect.track == TRACK)
    {
        return Ok(None);
    }
    let current_hash = catalogue
        .definition_hash(QUEST)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let id = fence.character_id.as_bytes().as_slice();
    let state: Option<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT pinned_content_revision, definition_hash FROM game_character_quest_states
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2",
    )
    .bind(id)
    .bind(QUEST)
    .fetch_optional(&mut **tx)
    .await?;
    let state_exists = state.is_some();
    let (pin, hash) = if let Some((pin, bytes)) = state {
        let hash = bytes
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if hash != current_hash
            && oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(catalogue, QUEST, &hash)
                .is_none()
        {
            return Ok(None);
        }
        (pin, hash)
    } else {
        if !capacity::new_state_fits(tx, fence.character_id).await? {
            return Ok(None);
        }
        (catalogue.content_revision().to_owned(), current_hash)
    };
    let (now, before): (i64, Option<i64>) = sqlx::query_as(
        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint,
         (SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2)",
    ).bind(id).bind(TRACK).fetch_one(&mut **tx).await?;
    if !capacity::tracks_fit(
        tx,
        fence.character_id,
        &[TRACK],
        usize::from(before.is_some()),
    )
    .await?
    {
        return Ok(None);
    }
    let stored = before.map(|v| (TRACK.to_owned(), v)).into_iter().collect();
    let Ok(changes) = catalogue.evaluate(transition, &stored, now) else {
        return Ok(None);
    };
    let request = QuestTransitionRequest {
        transition_key: key,
        cause: QuestCause::Command(command),
    };
    let binding = request_binding(fence.character_id, &request);
    let revision = persist::persist(
        tx,
        persist::QuestWrite {
            character: fence.character_id,
            root_revision: fence.expected_character_revision.get(),
            request: &request,
            transition,
            pin: &pin,
            hash: &hash,
            changes: &changes,
            binding: &binding,
            state_exists,
        },
    )
    .await?;
    Ok(Some(
        CharacterRevision::new(revision).map_err(|_| DurabilityError::InvalidStoredState)?,
    ))
}
