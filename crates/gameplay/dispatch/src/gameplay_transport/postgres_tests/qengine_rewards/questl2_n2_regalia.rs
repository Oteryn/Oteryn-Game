//! L2 Regalia, alliance supplies and tome progression enter through native commands.
use super::*;

fn probe(case: &'static str, required: &[&str]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::fixture::run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_quest_reward(admin, "l2_knowledge_binding", true).await?;
        let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("POSTGRES_CASE_STARTED {case}-binding server_version={version}");
        let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
        let present = required
            .iter()
            .all(|key| callbacks.iter().any(|c| c.key.ends_with(key)));
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_BINDING_PROBE {case} duration={:?}",
            started.elapsed()
        );
        assert!(
            present,
            "complete native journey bindings absent: {required:?}"
        );
        Ok(())
    })?;
    scenario(case)
}

#[test]
fn questl2_knowledge_batch_kilmaresh_regalia_full_journey() -> TestResult {
    probe("qengine-rewards-l2-regalia", &["/l2-regalia-combine"])
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with("/l2-regalia-combine"))
        .ok_or("Yonan source exchange")?;
    assert!(callback.is_paid_exchange());
    assert!(callback.storage.is_empty());
    let [npc] =
        super::super::classic_quests::install(owner, admitted, ["oteryn:npc.yonan"]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("Regalia capabilities")?;
    assert_eq!(
        wire_reward(owner, admitted, npc, &callback, false).await?.0,
        NpcIntentDisposition::Rejected
    );
    let position = Position {
        x: 33293,
        y: 31677,
        floor: -15,
    };
    for (n, item) in [31572, 31573, 31574].into_iter().enumerate() {
        supply(owner, admitted, item, 16000 + u64::try_from(n)?, position).await?;
    }
    assert_eq!(
        wire_reward(owner, admitted, npc, &callback, false).await?.0,
        NpcIntentDisposition::Rejected
    );
    for item in 31572..=31574 {
        assert_eq!(
            quantity(h, item).await?,
            1,
            "missing fourth piece preserves all inputs"
        );
    }
    assert_eq!(quantity(h, 31576).await?, 0);
    supply(owner, admitted, 31575, 16003, position).await?;
    let (disposition, _, command) = wire_reward(owner, admitted, npc, &callback, false).await?;
    assert_eq!(disposition, NpcIntentDisposition::Ok);
    for item in 31572..=31575 {
        assert_eq!(quantity(h, item).await?, 0);
    }
    assert_eq!(quantity(h, 31576).await?, 1);
    let before = counts(h).await?;
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: "yes".into(),
    })
    .map_err(debug)?;
    let replay =
        super::super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        counts(h).await?,
        before,
        "literal replay cannot burn or mint again"
    );
    for (n, item) in [31572, 31573, 31574, 31575].into_iter().enumerate() {
        supply(owner, admitted, item, 16010 + u64::try_from(n)?, position).await?;
    }
    assert_eq!(
        wire_reward(owner, admitted, npc, &callback, false).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        quantity(h, 31576).await?,
        2,
        "a new paid occurrence is allowed without artificial progress"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_tracks")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    Ok(())
}

async fn supply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u32,
    n: u64,
    position: Position,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "supply fence")?
        .ok_or("supply fence")?;
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position,
            item: format!("oteryn:item.tibia.i{item}"),
            count: 1,
        },
        n,
        true,
    )
    .await
}


async fn quantity(h:&Harness,item:u32)->TestResult<i64>{
 Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1").bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
