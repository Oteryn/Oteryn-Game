//! Pinned Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
//! data-global/npc/gamon.lua:59-71, sha256:466de8702c51fc9c550f3f9f75e0301713eeec3a81bb8974fc848f10df5ead83.
//! No mission progress is seeded: Quentin issues the actual source introduction.
use super::*;

pub(super) async fn cash(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity*CASE definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::bigint FROM game_item_instances WHERE lifecycle=1").fetch_one(&h.pool).await?)
}
pub(super) async fn stakes(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i5941'").fetch_one(&h.pool).await?)
}
pub(super) async fn stage(h: &Harness) -> TestResult<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u7_8/friends_and_traders/the_blessed_stake'").fetch_optional(&h.pool).await?)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [quentin, gamon] = super::super::classic_quests::install(
        owner,
        admitted,
        ["oteryn:npc.quentin", "oteryn:npc.gamon"],
    )
    .await?;
    exercise_with_npcs(owner, admitted, h, quentin, gamon).await
}

pub(super) async fn exercise_with_npcs(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    quentin: EntityRef,
    gamon: EntityRef,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_CASE_STARTED paid-stake server_version={version}");
    assert_eq!(stage(h).await?, None);
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let introduction = callbacks
        .iter()
        .find(|c| c.key.ends_with("exchange-quentin-start"))
        .ok_or("Quentin introduction binding")?;
    let purchase = callbacks
        .iter()
        .find(|c| c.key.ends_with("exchange-gamon-stake"))
        .ok_or("Gamon purchase binding")?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    assert_eq!(
        wire_grant(owner, admitted, quentin, introduction).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(stage(h).await?, Some(1));
    assert_eq!(cash(h).await?, 0);
    assert_eq!(stakes(h).await?, 0);
    assert_eq!(
        wire_grant(owner, admitted, gamon, purchase).await?.0,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(cash(h).await?, 0);
    assert_eq!(stakes(h).await?, 0);
    assert_eq!(stage(h).await?, Some(1));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: Position {
                x: u16::try_from(at.x)?,
                y: u16::try_from(at.y)?,
                floor: -i8::try_from(at.floor)?,
            },
            item: "oteryn:item.tibia.i3043".into(),
            count: 1,
        },
        2000,
        true,
    )
    .await?;
    assert_eq!(cash(h).await?, 10000);
    let (disposition, _, last) = wire_grant(owner, admitted, gamon, purchase).await?;
    assert_eq!(disposition, NpcIntentDisposition::Ok);
    assert_eq!(cash(h).await?, 5000);
    assert_eq!(stakes(h).await?, 1);
    assert_eq!(stage(h).await?, Some(1));
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: gamon,
        text: "yes".into(),
    })
    .map_err(debug)?;
    let replay =
        super::super::quest_world_callbacks::wire(owner, admitted, last, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(cash(h).await?, 5000);
    assert_eq!(stakes(h).await?, 1);
    assert_eq!(
        wire_grant(owner, admitted, gamon, purchase).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(cash(h).await?, 0);
    assert_eq!(stakes(h).await?, 2);
    assert_eq!(stage(h).await?, Some(1));
    let fees:(i64,i64,i64)=sqlx::query_as("SELECT count(*),coalesce(sum(fee_gold_units),0)::bigint,coalesce(sum(bank_debit_gold_units),0)::bigint FROM game_item_fee_burns WHERE cause_kind=18").fetch_one(&h.pool).await?;
    assert_eq!(fees, (2, 10000, 0));
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_fees")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 2);
    assert_eq!(
        wire_grant(owner, admitted, gamon, purchase).await?.0,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(cash(h).await?, 0);
    assert_eq!(stakes(h).await?, 2);
    Ok(())
}
