use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    use oteryn_gameplay_quests::quest::world_callbacks::Offering;

    let (key, quest, input, output, exchange) = if name.ends_with("billy") {
        (
            "oteryn:npc.billy",
            "oteryn:quest.small_health_potion",
            "oteryn:item.tibia.i3466",
            "oteryn:item.tibia.i7876",
            &["pan", "yes"][..],
        )
    } else {
        (
            "oteryn:npc.lee_delle",
            "oteryn:quest.studded_legs_quest",
            "oteryn:item.tibia.i2984",
            "oteryn:item.tibia.i3362",
            &["honey flower"][..],
        )
    };
    let [npc] = super::classic_quests_fixtures::install(owner, admitted, [key]).await?;
    super::classic_quests_fixtures::talk(owner, admitted, npc, 41, "hi").await?;
    let (&confirmation, offer) = exchange.split_last().ok_or("exchange word")?;
    for (id, word) in (42..).zip(offer) {
        super::classic_quests_fixtures::talk(owner, admitted, npc, id, word).await?;
    }
    let mut command = 42 + u64::try_from(offer.len())?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let refusal_result = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: confirmation.into(),
            },
        )
        .await;
    command += 1;
    assert!(!refusal_result.unknown);
    assert_eq!(refusal_result.disposition, NpcIntentDisposition::Rejected);
    let lines = refusal_result.conversation.ok_or("refusal reply")?.lines;
    let refusal = if name.ends_with("billy") {
        "You don't have a pan."
    } else {
        "Honey flowers are my favourite."
    };
    assert_eq!(lines, [refusal]);
    let count = |item: &'static str| {
        sqlx::query_scalar::<_, i64>("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(item)
            .fetch_one(&h.pool)
    };
    assert_eq!(count(output).await?, 0);
    // Fresh inputs keep the source trade repeatable after the first completion.
    for attempt in 1..=2 {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("quest fence")?;
        super::quest_world_callbacks_fixtures::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: input.into(),
                count: 1,
            },
            52000 + attempt,
            true,
        )
        .await?;
        for (id, word) in (command..).zip(exchange) {
            super::classic_quests_fixtures::talk(owner, admitted, npc, id, word).await?;
        }
        command += u64::try_from(exchange.len())?;
        assert_eq!(count(input).await?, 0);
        assert_eq!(count(output).await?, i64::try_from(attempt)?);
    }
    let completed: bool = sqlx::query_scalar(
        "SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key=$1",
    ).bind(quest).fetch_one(&h.pool).await?;
    assert!(completed, "the first exchange completes the quest");
    Ok(())
}
