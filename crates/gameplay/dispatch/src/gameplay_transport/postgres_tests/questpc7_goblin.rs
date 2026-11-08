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

// Exact sign sites from Task:Rafzan oldid401277; source B3 witnesses are
// independently checked by authoring tests before these fixture cells are used.
const SIGN_SITES: [Position; 3] = [
    Position { x: 33055, y: 32091, floor: -7 },
    Position { x: 33026, y: 32091, floor: -7 },
    Position { x: 33032, y: 32083, floor: -7 },
];
const SIGN_DECOY: Position = Position { x: 33055, y: 32089, floor: -7 };

#[test]
fn questpc7_goblin_three_personal_sign_sites() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-goblin-signs", || {
        super::quest_world_callbacks::scenario("qengine-rewards-goblin-signs",
            Position { x: 33055, y: 32090, floor: -7 }, false)
    });
    eprintln!("QUESTPC7_GOBLIN_SIGNS POSTGRES_CASE_COMPLETED duration={:?} success={}", started.elapsed(), result.is_ok());
    result
}

pub(super) fn sign_objects() -> Vec<(Position, u16)> {
    // A fixture corridor, not an asserted source map layout. Target coordinates,
    // appearances and native Item bindings are exact independently audited facts.
    let mut objects = Vec::new();
    for x in 33025..=33056 { for y in 32082..=32093 {
        objects.push((Position { x, y, floor: -7 }, 100));
    }}
    // Native targets are added only by sign_native_items, never duplicated as terrain.
    objects
}

pub(super) fn sign_native_items() -> Vec<(Position, u32)> {
    SIGN_SITES.into_iter().chain([SIGN_DECOY]).map(|p| (p, 17679)).collect()
}

pub(super) async fn exercise_signs(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};
    use super::questpc7_ferumbras::{use_at, value, walk};
    let version: String = sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_GOBLIN_SIGNS POSTGRES_STARTED version={version}");
    assert_eq!(use_at(owner, admitted, SIGN_SITES[0], 17679).await?, UseDisposition::Rejected);
    assert_eq!(receipts(h).await?, 0);
    exercise(owner, admitted, h).await?;
    assert_eq!(use_at(owner, admitted, SIGN_DECOY, 17679).await?, UseDisposition::NothingToUse);
    assert_eq!(quantity(h, 17668).await?, 3, "same appearance elsewhere cannot consume a sign");
    assert_eq!(receipts(h).await?, 2);
    let approaches = [
        Position { x: 33054, y: 32090, floor: -7 },
        Position { x: 33027, y: 32090, floor: -7 },
        Position { x: 33031, y: 32083, floor: -7 },
    ];
    for (index, (site, approach)) in SIGN_SITES.into_iter().zip(approaches).enumerate() {
        walk(owner, admitted, approach).await?;
        let command = admitted.continuity.next_command_id;
        let payload = super::questpc7_hoist_story::target(owner, site, 17679)?;
        assert_eq!(use_at(owner, admitted, site, 17679).await?, UseDisposition::Committed);
        assert_eq!(quantity(h, 17668).await?, 2 - i64::try_from(index)?);
        assert_eq!(value(owner, admitted, &format!("{ROOT}signs")).await?, i64::try_from(index + 1)?);
        assert_eq!(value(owner, admitted, &format!("{ROOT}sign-site-{}", index + 1)).await?, 1);
        let expected_receipts = 3 + i64::try_from(index)?;
        assert_eq!(receipts(h).await?, expected_receipts);
        let replay = super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
        assert_eq!(decode_use_result(&replay).map_err(debug)?, UseDisposition::Committed);
        assert_eq!(receipts(h).await?, expected_receipts, "exact command replay has no new effect");
        assert_eq!(use_at(owner, admitted, site, 17679).await?, UseDisposition::Rejected);
        assert_eq!(quantity(h, 17668).await?, 2 - i64::try_from(index)?);
        assert_eq!(receipts(h).await?, expected_receipts, "fresh duplicate visit cannot spend another sign");
    }
    assert_eq!(money(h).await?, 8875);
    assert_eq!(value(owner, admitted, &format!("{ROOT}programme")).await?, 0);
    for suffix in ["odours", "captures", "urine", "trunks", "stalkers"] {
        assert_eq!(value(owner, admitted, &format!("{ROOT}{suffix}")).await?, 0);
    }
    for (item, count) in TOOLS.into_iter().skip(1) { assert_eq!(quantity(h, item).await?, count); }
    let finished: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(!finished, "three signs do not complete the six-job programme");
    Ok(())
}
