//! Mission 1 advances only on the corresponding completed NPC passage.
use super::persist::{QuestWrite, persist};
use super::*;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
};

const QUEST: &str = "oteryn:quest.the_postman_missions_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission01";

fn leg(route: &str) -> Option<(i64, &'static str)> {
    // Crystal 201189f8, data-global/npc: captain_bluebear.lua:70-73,
    // uzon.lua:80-83, captain_seahorse.lua:60-63, brodrosch.lua:125-128.
    match route {
        "oteryn:service.travel.captain_bluebear/carlin" => {
            Some((1, "oteryn:quest-travel/postman/bluebear-carlin"))
        }
        "oteryn:service.travel.uzon/edron" => Some((2, "oteryn:quest-travel/postman/uzon-edron")),
        "oteryn:service.travel.captain_seahorse/venore" => {
            Some((3, "oteryn:quest-travel/postman/seahorse-venore"))
        }
        "oteryn:service.travel.brodrosch/cormaya" => {
            Some((4, "oteryn:quest-travel/postman/brodrosch-cormaya"))
        }
        _ => None,
    }
}

pub(in crate::durability) async fn complete(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    receipt: &sqlx::postgres::PgRow,
) -> StoredResult<()> {
    let route: &str = receipt.try_get("route")?;
    let Some((before, key)) = leg(route) else {
        return Ok(());
    };
    let root = assert_gameplay_fence(tx, &fence, node)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let character = fence.character_id;
    let Some(state) = sqlx::query(
        "SELECT s.pinned_content_revision, s.definition_hash
           FROM game_character_quest_states s
           JOIN game_character_quest_tracks t USING (character_id, quest_key)
          WHERE s.character_id=encode($1,'hex')::uuid AND s.quest_key=$2
            AND t.track_key=$3 AND t.value=$4",
    )
    .bind(character.as_bytes().as_slice())
    .bind(QUEST)
    .bind(TRACK)
    .bind(before)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(());
    };

    let request = QuestTransitionRequest {
        transition_key: key.into(),
        cause: QuestCause::stored(
            "command",
            uuid_text(receipt.try_get::<&str, _>("game_session_id")?)?,
            numeric_u64(receipt, "command_id")?,
        )?,
    };
    let transition = QuestTransition {
        key: key.into(),
        quest: QUEST.into(),
        effects: vec![QuestEffect {
            track: TRACK.into(),
            from: QuestComparison::Eq(before),
            effect: QuestEffectKind::Set(before + 1),
        }],
        completes: false,
        experience: None,
        world_change: None,
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
            transition: &transition,
            pin: state.try_get("pinned_content_revision")?,
            hash: &hash,
            changes: &[QuestTrackChange {
                track: TRACK.into(),
                before,
                after: before + 1,
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
    use super::leg;

    #[test]
    fn postman_legs_bind_carrier_and_destination_in_source_order() {
        for (index, route) in [
            "oteryn:service.travel.captain_bluebear/carlin",
            "oteryn:service.travel.uzon/edron",
            "oteryn:service.travel.captain_seahorse/venore",
            "oteryn:service.travel.brodrosch/cormaya",
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(leg(route).map(|(stage, _)| stage), Some(index as i64 + 1));
        }
        assert_eq!(leg("oteryn:service.travel.captain_bluebear/edron"), None);
        assert_eq!(leg("oteryn:service.travel.captain_seahorse/carlin"), None);
    }
}
