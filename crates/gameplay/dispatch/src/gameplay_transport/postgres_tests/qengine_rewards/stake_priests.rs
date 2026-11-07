//! Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d: data-global/npc/{priest}.lua.
//! Every binding retains its exact donor SHA; questl2_exchange_prayers validates
//! the source prayer, offer, stage and carried stake. Tyrias alone consumes it.
use super::*;

const PRIESTS: [&str; 10] = [
    "quentin", "tibra", "maealil", "yberius", "isimov", "amanda", "kasmir", "rahkem", "brewster",
    "tyrias",
];
async fn wooden_ids(h: &Harness) -> TestResult<Vec<sqlx::types::Uuid>> {
    Ok(sqlx::query_scalar("SELECT item_instance_id FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i5941' ORDER BY item_instance_id").fetch_all(&h.pool).await?)
}
async fn blessed(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i5942'").fetch_one(&h.pool).await?)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let npcs = super::super::classic_quests::install_nearby(
        owner,
        admitted,
        [
            "oteryn:npc.quentin",
            "oteryn:npc.tibra",
            "oteryn:npc.maealil",
            "oteryn:npc.yberius",
            "oteryn:npc.isimov",
            "oteryn:npc.amanda",
            "oteryn:npc.kasmir",
            "oteryn:npc.rahkem",
            "oteryn:npc.brewster",
            "oteryn:npc.tyrias",
            "oteryn:npc.gamon",
        ],
    )
    .await?;
    // Walk the actual wire corridor to the centre of the two NPC rows.
    for _ in 0..2 {
        use oteryn_protocol_oteryn::world_spatial::{
            StepDirection, StepDisposition, decode_step_result, encode_step_intent,
        };
        let command = admitted.continuity.next_command_id;
        let result = super::super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            1,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        assert_eq!(
            decode_step_result(&result).map_err(debug)?,
            StepDisposition::Moved
        );
    }
    paid_stake::exercise_with_npcs(owner, admitted, h, npcs[0], npcs[10]).await?;
    let original = wooden_ids(h).await?;
    assert_eq!(original.len(), 2);
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let final_prayer = callbacks
        .iter()
        .find(|c| c.key.ends_with("exchange-prayer-tyrias"))
        .ok_or("Tyrias final prayer binding")?;
    assert_eq!(
        wire_grant(owner, admitted, npcs[9], final_prayer).await?.0,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(paid_stake::stage(h).await?, Some(1));
    assert_eq!(wooden_ids(h).await?, original);
    assert_eq!(blessed(h).await?, 0);
    let mut last = 0;
    for (index, priest) in PRIESTS.iter().enumerate() {
        let key = format!("exchange-prayer-{priest}");
        let callback = callbacks
            .iter()
            .find(|c| c.key.ends_with(&key))
            .ok_or("ordered priest binding")?;
        let (result, _, command) = wire_grant(owner, admitted, npcs[index], callback).await?;
        assert_eq!(result, NpcIntentDisposition::Ok, "priest {priest}");
        assert_eq!(paid_stake::stage(h).await?, Some(i64::try_from(index)? + 2));
        if index < 9 {
            assert_eq!(
                wooden_ids(h).await?,
                original,
                "prayer retains original stake instances"
            );
            assert_eq!(blessed(h).await?, 0);
        }
        last = command;
    }
    assert_eq!(paid_stake::stakes(h).await?, 1);
    assert_eq!(blessed(h).await?, 1);
    assert_eq!(paid_stake::cash(h).await?, 0);
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npcs[9],
        text: "yes".into(),
    })
    .map_err(debug)?;
    let replay =
        super::super::quest_world_callbacks::wire(owner, admitted, last, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(paid_stake::stakes(h).await?, 1);
    assert_eq!(blessed(h).await?, 1);
    assert_eq!(
        wire_grant(owner, admitted, npcs[9], final_prayer).await?.0,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(paid_stake::stage(h).await?, Some(11));
    assert_eq!(blessed(h).await?, 1);
    Ok(())
}
