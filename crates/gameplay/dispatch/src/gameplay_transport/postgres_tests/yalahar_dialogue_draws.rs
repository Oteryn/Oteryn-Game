//! Crystal 201189f8 data-global/npc/telas.lua:94-105,385-399 and
//! data-global/npc/golem_servant.lua:84-108,203-208,238-240.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::WorldId;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.fixture_yalahar_draws";
const STAGE: &str = "oteryn:quest-progress/fixture-yalahar/request";
const DRAW: [&str; 3] = [
    "oteryn:quest-progress/fixture-yalahar/draw-one",
    "oteryn:quest-progress/fixture-yalahar/draw-two",
    "oteryn:quest-progress/fixture-yalahar/draw-three",
];
const TELAS: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/telas.lua:94-105,385-399";
const GOLEM: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/golem_servant.lua:84-108,203-208,238-240";
const PART_LINES: [&str; 12] = [
    "Essential component: bag of screws is missing!",
    "Spare part malfunctioning! Immediate replacement required!",
    "Repair procedure requires giant screwdriver. Hurry, flesh unit!",
    "V-belt irreparably damaged. Replacement needed immediately!",
    "Yalahari gear wheel stripped! Replacement needed at once!",
    "Energy net required immediately!",
    "Grease! Grease! Procedure impossible without proper amount of grease!",
    "Green power core depleted! Immediate replacement required!",
    "Several parts instable. Wonder glue infusion necessary.",
    "Danger! Danger! Elemental water urgently needed for cooling!",
    "Red power core failing! Immediate replacement required!",
    "Chaos matter reserves critical! Refill needed immediately!",
];

#[test]
fn telas_dialogue_persists_three_distinct_ingredient_indices() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| exercise(&admin, false).await)
}

#[test]
fn golem_servant_dialogue_reports_the_persisted_part_for_each_request() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| exercise(&admin, true).await)
}

fn binding(npc: &str, repair: bool) -> TestResult<QuestStateCatalogue> {
    let source = if repair { GOLEM } else { TELAS };
    let keys = if repair { &DRAW[..1] } else { &DRAW[..] };
    let sides = if repair { 12 } else { 6 };
    let stages: Vec<_> = (0..=u8::from(repair))
        .map(|stage| {
            let ops: Vec<Value> = keys.iter().map(|track| json!({
                "track": track,
                "from": if repair { json!({"kind": "any"}) } else { json!({"kind": "between", "minimum": -1, "maximum": 0}) },
                "effect": {"kind": "draw", "sides": sides}
            })).collect();
            json!({
                "key": format!("oteryn:quest-stage/fixture-yalahar/request-{stage}"),
                "completes": false,
                "trigger": {"kind": "npc_dialogue", "callback": {
                    "key": format!("oteryn:quest-reward/fixture-yalahar/request-{stage}"),
                    "quest": QUEST, "npc": npc,
                    "keywords": [if repair { "yes" } else { "mission" }],
                    "topic": 0, "next_topic": 0,
                    "storage": [{"track": STAGE, "minimum": stage, "maximum": stage, "set": stage + 1}],
                    "state_ops": ops,
                    "take": [],
                    "reply": if repair { Vec::new() } else { vec!["Specifically, three items need to be replaced. Are you ready for your first mission?"] },
                    "draw_replies": if repair { PART_LINES.to_vec() } else { Vec::new() },
                    "sources": [source]
                }}
            })
        }).collect();
    let tracks: Vec<_> =
        std::iter::once(json!({"key": STAGE, "initial": 0, "minimum": 0, "maximum": 2}))
            .chain(
                keys.iter().map(
                    |track| json!({"key": track, "initial": -1, "minimum": -1, "maximum": sides}),
                ),
            )
            .collect();
    let binding = json!({"schema": 1, "quests": [{
        "quest": QUEST, "recipe_sha256": format!("{:x}", Sha256::digest(source)),
        "sources": [source], "tracks": tracks, "stages": stages
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&binding)?)?.attach(&base)?)
}

async fn exercise(admin: &str, repair: bool) -> TestResult {
    let h =
        Harness::create_with_admission_seed(admin, "yalahar_dialogue_draws", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
        WorldId::decode(&id(42)).map_err(debug)?,
        "content-1",
    )?;
    let room = pin.qualified_room().clone();
    let deps = Dependencies::with_content(
        pin.into_channel_pin(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let npc_key = if repair {
        "oteryn:npc.golem_servant"
    } else {
        "oteryn:npc.telas"
    };
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = controller.active();
    owner.quest_catalogue = Some(Arc::new(binding(npc_key, repair)?));
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    deps.initialize(&owner).await;
    owner.admit_quest_session(&admitted).await;
    let [npc] = install(&owner, &admitted, [npc_key]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest session")?;
    talk(&mut owner, &admitted, npc, 1, "hi").await?;
    let lines = talk(
        &mut owner,
        &admitted,
        npc,
        2,
        if repair { "yes" } else { "mission" },
    )
    .await?;
    let state = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(STAGE), Some(&1));
    let first = *state.tracks().get(DRAW[0]).ok_or("first draw")?;
    if repair {
        assert!((1..=12).contains(&first));
        assert_eq!(lines, [PART_LINES[usize::try_from(first - 1)?]]);
        let lines = talk(&mut owner, &admitted, npc, 3, "yes").await?;
        let state = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(state.tracks().get(STAGE), Some(&2));
        let next = *state.tracks().get(DRAW[0]).ok_or("next part")?;
        assert!((1..=12).contains(&next));
        assert_eq!(lines, [PART_LINES[usize::try_from(next - 1)?]]);
    } else {
        let chosen = DRAW
            .iter()
            .map(|key| state.tracks().get(*key).copied().ok_or("ingredient draw"))
            .collect::<Result<BTreeSet<_>, _>>()?;
        assert_eq!(chosen.len(), 3);
        assert!(chosen.iter().all(|index| (1..=6).contains(index)));
    }
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-reward/fixture-yalahar/request-%'"
    ).fetch_one(&h.pool).await?;
    assert_eq!(receipts, if repair { 2 } else { 1 });
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
