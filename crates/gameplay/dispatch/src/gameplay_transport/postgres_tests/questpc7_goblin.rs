//! Rafzan's chosen six-job start and exact source-priced tool bundle, without progress seeds.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

const QUEST: &str = "oteryn:quest.goblin_merchant_quest";
const ROOT: &str = "oteryn:quest-progress/questpc7/goblin/";
const TOOLS: [(u32, i64); 5] = [(17668, 3), (17669, 1), (17670, 1), (17671, 1), (17676, 1)];

#[test]
fn questpc7_goblin_six_jobs_and_exact_paid_tools() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-goblin-intro", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-goblin-intro",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_GOBLIN_INTRO POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn conversation(name: &str, word: &str) -> TestResult<RewardCallback> {
    // Only supplies conversation words to the shared wire driver; it does not
    // add a transition to the runtime catalogue or write quest state.
    Ok(serde_json::from_value(serde_json::json!({
        "key":format!("oteryn:quest-reward/questpc7-goblin-{name}"),
        "quest":QUEST,"npc":"oteryn:npc.rafzan","keywords":[word],
        "topic":0,"next_topic":0,"storage":[],"take":[],"give":null,
        "outfit":null,"achievement":null,"reply":[],"sources":[]
    }))?)
}

async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2")
        .bind(id(41).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?)
}
async fn money(h: &Harness) -> TestResult<i64> {
    Ok(
        quantity(h, 3031).await?
            + quantity(h, 3035).await? * 100
            + quantity(h, 3043).await? * 10000,
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_GOBLIN_INTRO POSTGRES_STARTED version={version}");
    let mut selected = a.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 4, 6, 11, 12, 14, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    a.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let [rafzan] = super::classic_quests::install(owner, a, ["oteryn:npc.rafzan"]).await?;
    assert_eq!(receipts(h).await?, 0);
    let intro = conversation("intro", "swamp jobs")?;
    let (result, _, last) =
        super::qengine_rewards::wire_reward(owner, a, rafzan, &intro, false).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    for suffix in [
        "programme",
        "signs",
        "odours",
        "captures",
        "urine",
        "trunks",
        "stalkers",
    ] {
        assert_eq!(
            super::questpc7_ferumbras::value(owner, a, &format!("{ROOT}{suffix}")).await?,
            0,
            "fresh accepted job {suffix}"
        );
    }
    assert_eq!(receipts(h).await?, 1, "one atomic seven-track start");
    for (item, _) in TOOLS {
        assert_eq!(
            quantity(h, item).await?,
            0,
            "jobs do not manufacture merchant stock"
        );
    }
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: rafzan,
        text: "swamp jobs".into(),
    })
    .map_err(debug)?;
    let replay = super::quest_world_callbacks::wire(owner, a, last, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(receipts(h).await?, 1);
    let kit = conversation("kit", "buy swamp kit")?;
    let (poor, _, _) = super::qengine_rewards::wire_reward(owner, a, rafzan, &kit, false).await?;
    assert_eq!(poor, NpcIntentDisposition::Rejected);
    assert_eq!(receipts(h).await?, 1);
    for (item, _) in TOOLS {
        assert_eq!(quantity(h, item).await?, 0);
    }
    let fence = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: "oteryn:item.tibia.i3043".into(),
            count: 1,
        },
        33000,
        true,
    )
    .await?;
    assert_eq!(money(h).await?, 10000);
    let (paid, _, last) =
        super::qengine_rewards::wire_reward(owner, a, rafzan, &kit, false).await?;
    assert_eq!(paid, NpcIntentDisposition::Ok);
    assert_eq!(
        money(h).await?,
        8875,
        "3x75+200+400+150+150 exact donor prices"
    );
    for (item, count) in TOOLS {
        assert_eq!(quantity(h, item).await?, count);
    }
    assert_eq!(receipts(h).await?, 2);
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: rafzan,
        text: "buy swamp kit".into(),
    })
    .map_err(debug)?;
    let replay = super::quest_world_callbacks::wire(owner, a, last, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(money(h).await?, 8875);
    for (item, count) in TOOLS {
        assert_eq!(quantity(h, item).await?, count);
    }
    assert_eq!(receipts(h).await?, 2);
    let unfinished: bool=sqlx::query_scalar("SELECT completed_character_revision IS NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(
        unfinished,
        "introduction and shopping do not complete the six jobs"
    );
    Ok(())
}
