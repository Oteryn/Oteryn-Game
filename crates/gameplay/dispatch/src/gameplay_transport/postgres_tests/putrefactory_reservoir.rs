//! Crystal 201189f8, rotten_blood_quest/creaturescripts_putrefactory-x.lua:155-224.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{scenario, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use serde_json::json;
use std::sync::Arc;

pub(super) const LEGACY: &str = "putrefactory-reservoir-legacy";
pub(super) const CAP: &str = "putrefactory-reservoir-cap";
const QUEST: &str = "oteryn:quest/rotten-blood";
const CHARGE: &str =
    "oteryn:quest-progress/crystalserver/quest/u13_20/rotten_blood/putrefactory_task";
const CLOCK: &str = "oteryn:quest-progress/rotten-blood/putrefactory/reservoir-1-used-at";
const STARTED_AT: i64 = 1_800_000_000;
const AT: Position = Position {
    x: 34081,
    y: 31704,
    floor: -14,
};

#[test]
fn putrefactory_reservoir_recharges_unset_counter_and_cools_down_on_use() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(LEGACY, || scenario(LEGACY, AT, false))
}

#[test]
fn putrefactory_reservoir_caps_charge_without_restarting_clock_when_full() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run(CAP, || scenario(CAP, AT, false))
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/fixture/putrefactory-reservoir-1",
        "quest":QUEST,
        "trigger":"use",
        "actor_kind":"player",
        "selector":{"position":AT,"action":47953},
        "passages":[],
        "storage":[],
        "offerings":[],
        "return_on_failure":false,
        "reversible":false,
        "requires_world_admission":true,
        "sources":["Crystal 201189f8, rotten_blood_quest/creaturescripts_putrefactory-x.lua:155-224"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    cap: bool,
) -> TestResult {
    let mut reservoir = serde_json::to_value(callback()?)?;
    reservoir["effects"]["magic"] = json!({"position":AT,"effect":241});
    reservoir["state_operations"] = json!([
        {"track":CHARGE,"from":{"kind":"between","minimum":-1,"maximum":99},
            "effect":{"kind":"recharge","amount":20,"empty":0,"capacity":100}},
        {"track":CLOCK,"from":{"kind":"elapsed_at_least","seconds":20},
            "effect":{"kind":"set_now"}}
    ]);
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,
        "quests":[{
            "quest":QUEST,
            "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
            "sources":["Crystal 201189f8, rotten_blood_quest/creaturescripts_putrefactory-x.lua:155-224"],
            "tracks":[
                {"key":CHARGE,"initial":if cap { 93 } else { -1 },"minimum":-1,"maximum":2147483647},
                {"key":CLOCK,"initial":-1,"minimum":-1,"maximum":i64::MAX}
            ],
            "stages":[{"key":"reservoir-1","completes":false,
                "trigger":{"kind":"map_use","callback":reservoir}}]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("test catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    h.root.set_test_quest_clock(STARTED_AT).map_err(debug)?;
    let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
        .ok_or("reservoir placement")?;
    let mut placement = owner.bundle_map().ok_or("map")?.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let tracks = || async {
        sqlx::query_as::<_, (String, i64)>(
            "SELECT track_key,value FROM game_character_quest_tracks WHERE quest_key=$1 ORDER BY track_key",
        )
        .bind(QUEST)
        .fetch_all(&h.pool)
        .await
    };
    let reply = wire(owner, admitted, 1, 2, payload.clone()).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
    let values = tracks().await?;
    assert_eq!(values.len(), 2);
    assert_eq!(values[0], (CHARGE.to_owned(), if cap { 100 } else { 20 }));
    assert_eq!(values[1].0, CLOCK);
    assert_eq!(values[1].1, STARTED_AT);

    assert_eq!(wire(owner, admitted, 1, 2, payload.clone()).await?, reply);
    h.root.advance_test_quest_clock(19).map_err(debug)?;
    let reply = wire(owner, admitted, 2, 2, payload.clone()).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Rejected);
    assert_eq!(tracks().await?, values, "refused use and replay preserve both tracks");
    h.root.advance_test_quest_clock(1).map_err(debug)?;
    let reply = wire(owner, admitted, 3, 2, payload).await?;
    let disposition = decode_use_result(&reply).map_err(debug)?;
    if cap {
        assert_eq!(disposition, UseDisposition::Rejected);
        assert_eq!(tracks().await?, values, "full charge leaves the expired clock alone");
    } else {
        assert_eq!(disposition, UseDisposition::Committed);
        assert_eq!(
            tracks().await?,
            vec![(CHARGE.to_owned(), 40), (CLOCK.to_owned(), STARTED_AT + 20)]
        );
    }
    Ok(())
}
