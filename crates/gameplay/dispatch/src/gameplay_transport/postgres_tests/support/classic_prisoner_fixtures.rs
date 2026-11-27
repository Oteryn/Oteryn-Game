use super::fixture::{Harness, debug};
use super::quest_world_callbacks_fixtures::wire;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::npc_service::{
    CAPABILITY_NPC_SERVICE_V1, COMMAND_TYPE_NPC_TALK_INTENT, NpcIntentDisposition, NpcTalkIntent,
    decode_npc_intent_result, encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    word: &str,
    replay: bool,
) -> TestResult<NpcIntentDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let command = admitted.continuity.next_command_id;
    let intent = encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: word.into(),
    })
    .map_err(debug)?;
    let response = wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_NPC_TALK_INTENT,
        intent.clone(),
    )
    .await?;
    assert_eq!(admitted.continuity.next_command_id, command + 1);
    if replay {
        assert_eq!(
            wire(
                owner,
                admitted,
                command,
                COMMAND_TYPE_NPC_TALK_INTENT,
                intent
            )
            .await?,
            response
        );
        assert_eq!(admitted.continuity.next_command_id, command + 1);
    }
    Ok(decode_npc_intent_result(&response).map_err(debug)?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [npc] =
        super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.a_prisoner"]).await?;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[CAPABILITY_NPC_SERVICE_V1])
            .ok_or("Prisoner NPC service capability")?;
    let mut mint_command = 900;
    super::qengine_voucher_stories_fixtures::mint(owner, admitted, 3585, 14, &mut mint_command)
        .await?;
    assert_eq!(admitted.continuity.next_command_id, 1);
    for (word, topic, apples, key) in [
        ("hi", 0, 14, None),
        ("key", 0, 14, None),
        ("riddle", 1, 14, None),
        ("PD-D-KS-P-PD", 2, 14, None),
        ("yes", 3, 7, None),
        ("bye", 0, 7, None),
        ("hi", 0, 7, None),
        ("key", 3, 7, None),
        ("yes", 4, 7, None),
        ("yes", 5, 7, None),
        ("yes", 0, 7, Some(3666)),
    ] {
        assert_eq!(
            talk(owner, admitted, npc, word, key.is_some()).await?,
            NpcIntentDisposition::Ok,
            "{word}, expected topic {topic}",
        );
        assert_eq!(
            owner
                .states
                .npc_conversations
                .fixture_quest_topic(admitted.game_session_id)?,
            topic,
            "{word}"
        );
        let actual: i64 = sqlx::query_scalar(
            "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances
             WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3585'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(actual, apples);
        let numbers: Vec<i32> = sqlx::query_scalar(
            "SELECT k.key_number FROM game_item_keys k JOIN game_item_instances i USING(item_instance_id)
             WHERE i.definition_production_key='oteryn:item.tibia.i2969'",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(numbers, key.into_iter().collect::<Vec<_>>());
    }

    for word in [
        "bye",
        "hi",
        "riddle",
        "PD-D-KS-P-PD",
        "yes",
        "yes",
        "yes",
        "yes",
    ] {
        talk(owner, admitted, npc, word, false).await?;
        assert_eq!(
            owner
                .states
                .npc_conversations
                .fixture_quest_topic(admitted.game_session_id)?,
            0,
            "a prior key claim closes the riddle route ({word})",
        );
    }
    let receipts: Vec<String> = sqlx::query_scalar(
        "SELECT transition_key FROM game_quest_reward_sources
         WHERE plan->>'quest' = 'oteryn:quest.mad_mage_room_quest' ORDER BY transition_key",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        receipts,
        [
            "oteryn:quest-reward/l6-mad-mage-prisoner-apples",
            "oteryn:quest-reward/l6-mad-mage-prisoner-key"
        ],
        "resume, replay and the later approach leave one payment and one key claim",
    );
    Ok(())
}
