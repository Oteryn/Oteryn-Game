//! Mission 1 advances only on the corresponding completed NPC passage.
use super::persist::{QuestWrite, persist};
use super::*;
use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind};

const QUEST: &str = "oteryn:quest.the_postman_missions_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission01";

pub(in crate::durability) fn leg(route: &str) -> Option<&'static str> {
    // Crystal 201189f8, data-global/npc: captain_bluebear.lua:70-73,
    // uzon.lua:80-83, captain_seahorse.lua:60-63, brodrosch.lua:125-128.
    match route {
        "oteryn:service.travel.captain_bluebear/carlin" => Some("npc_2"),
        "oteryn:service.travel.uzon/edron" => Some("npc_6"),
        "oteryn:service.travel.captain_seahorse/venore" => Some("npc_3"),
        "oteryn:service.travel.brodrosch/cormaya" => Some("npc_1"),
        _ => None,
    }
}

fn source_catalogue(pin: &str) -> StoredResult<&'static QuestStateCatalogue> {
    // Arrival consumes transition definitions only. The receipt retains the
    // Character's own quest pin and definition hash.
    static SOURCE: std::sync::OnceLock<std::result::Result<QuestStateCatalogue, String>> =
        std::sync::OnceLock::new();
    SOURCE
        .get_or_init(|| {
            oteryn_gameplay_quests::quest::loader::parse_quest_state(
                include_str!("../../../../../../content/quests/missions/quest-state.json"),
                pin,
            )
            .map(|source| source.catalogue().clone())
            .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|_| DurabilityError::InvalidStoredState)
}

fn transition_key(leg: &str) -> String {
    format!("oteryn:quest-transition/quest/u7_24/the_postman_missions/mission01/{leg}")
}

pub(in crate::durability) async fn complete(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    receipt: &sqlx::postgres::PgRow,
) -> StoredResult<()> {
    let route: &str = receipt.try_get("route")?;
    let Some(leg) = leg(route) else {
        return Ok(());
    };
    let root = assert_gameplay_fence(tx, &fence, node)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let character = fence.character_id;
    let Some(state) = sqlx::query(
        "SELECT s.pinned_content_revision, s.definition_hash, t.value
           FROM game_character_quest_states s
           JOIN game_character_quest_tracks t USING (character_id, quest_key)
          WHERE s.character_id=encode($1,'hex')::uuid AND s.quest_key=$2
            AND t.track_key=$3",
    )
    .bind(character.as_bytes().as_slice())
    .bind(QUEST)
    .bind(TRACK)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(());
    };

    let pin = state.try_get("pinned_content_revision")?;
    let key = transition_key(leg);
    let catalogue = source_catalogue(pin)?;
    let transition = catalogue
        .transition(&key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let [
        QuestEffect {
            track,
            from: QuestComparison::Eq(before),
            effect: QuestEffectKind::Set(after),
        },
    ] = transition.effects.as_slice()
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    if track != TRACK
        || transition.quest != QUEST
        || transition.completes
        || transition.experience.is_some()
        || transition.world_change.is_some()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    if state.try_get::<i64, _>("value")? != *before {
        return Ok(());
    }
    let request = QuestTransitionRequest {
        transition_key: key,
        cause: QuestCause::stored(
            "command",
            uuid_text(receipt.try_get::<&str, _>("game_session_id")?)?,
            numeric_u64(receipt, "command_id")?,
        )?,
    };
    let hash = state
        .try_get::<Vec<u8>, _>("definition_hash")?
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    persist(
        tx,
        QuestWrite {
            character,
            root_revision: root.revision,
            request: &request,
            transition,
            pin,
            hash: &hash,
            changes: &[QuestTrackChange {
                track: TRACK.into(),
                before: *before,
                after: *after,
            }],
            binding: &request_binding(character, &request),
            state_exists: true,
        },
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postman_legs_bind_carrier_and_destination_in_source_order() -> StoredResult<()> {
        let catalogue = source_catalogue("content-1")?;
        for (index, route) in [
            "oteryn:service.travel.captain_bluebear/carlin",
            "oteryn:service.travel.uzon/edron",
            "oteryn:service.travel.captain_seahorse/venore",
            "oteryn:service.travel.brodrosch/cormaya",
        ]
        .into_iter()
        .enumerate()
        {
            let key = transition_key(leg(route).ok_or(DurabilityError::InvalidStoredState)?);
            let transition = catalogue
                .transition(&key)
                .ok_or(DurabilityError::InvalidStoredState)?;
            let stage = i64::try_from(index).map_err(|_| DurabilityError::InvalidStoredState)? + 1;
            assert_eq!(
                transition.effects,
                [QuestEffect {
                    track: TRACK.into(),
                    from: QuestComparison::Eq(stage),
                    effect: QuestEffectKind::Set(stage + 1),
                }]
            );
        }
        assert_eq!(leg("oteryn:service.travel.captain_bluebear/edron"), None);
        assert_eq!(leg("oteryn:service.travel.captain_seahorse/carlin"), None);
        Ok(())
    }
}
